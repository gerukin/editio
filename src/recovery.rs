//! App-owned, atomic recovery drafts. One worker and one replaceable pending job.
use editio::buffer::Buffer;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const DEBOUNCE: Duration = Duration::from_secs(30);
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    version: u8,
    pub path: Option<Vec<u8>>,
    pub format: Option<String>,
    crlf: bool,
    bom: bool,
    bytes: usize,
    checksum: u32,
    time: u64,
}
#[cfg(unix)]
fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}
#[cfg(windows)]
fn encode_path(path: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}
#[cfg(unix)]
fn decode_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(bytes).into()
}
#[cfg(windows)]
fn decode_path(bytes: &[u8]) -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(
        &bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .into()
}
fn identity(path: &Path) -> io::Result<PathBuf> {
    if let Ok(path) = fs::canonicalize(path) {
        return Ok(path);
    }
    let absolute = std::path::absolute(path)?;
    let parent = absolute.parent().unwrap_or(Path::new("."));
    Ok(fs::canonicalize(parent)
        .unwrap_or(parent.to_owned())
        .join(absolute.file_name().unwrap_or_default()))
}
fn root() -> io::Result<PathBuf> {
    std::env::home_dir()
        .filter(|p| p.is_absolute())
        .map(|p| p.join(".local/state/editio/recovery"))
        .ok_or_else(|| io::Error::other("Cannot locate recovery home directory"))
}
struct Lease {
    dir: PathBuf,
    _lock: Option<File>,
}
impl Lease {
    fn create(root: &Path) -> io::Result<Self> {
        let existing = root.ancestors().find(|p| p.exists()).map(Path::to_owned);
        fs::create_dir_all(root)?;
        for ancestor in root.ancestors() {
            sync_dir(ancestor)?;
            if existing.as_deref() == Some(ancestor) {
                break;
            }
        }
        let dir = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(root)?
            .keep();
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(dir.join("lock"))?;
        lock.try_lock().map_err(io::Error::other)?;
        Ok(Self {
            dir,
            _lock: Some(lock),
        })
    }
    fn claim(dir: PathBuf) -> io::Result<Self> {
        if fs::symlink_metadata(&dir)?.file_type().is_symlink()
            || fs::symlink_metadata(dir.join("lock"))?
                .file_type()
                .is_symlink()
        {
            return Err(io::Error::other("Invalid recovery path"));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.join("lock"))?;
        lock.try_lock().map_err(io::Error::other)?;
        Ok(Self {
            dir,
            _lock: Some(lock),
        })
    }
    fn remove_draft(&self) -> io::Result<()> {
        match fs::remove_file(self.dir.join("draft")) {
            Ok(()) => sync_dir(&self.dir),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        // Preserve every committed draft. Empty/orphan directories are harmless and
        // retried by discovery (Windows cannot remove an open lock file).
        self._lock.take();
        if !self.dir.join("draft").exists() {
            let _ = fs::remove_file(self.dir.join("lock"));
            let _ = fs::remove_dir(&self.dir);
        }
    }
}
fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn open_draft(dir: &Path) -> io::Result<File> {
    let path = dir.join("draft");
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(io::Error::other(
            "Recovery draft must not be a symbolic link",
        ));
    }
    tapp_ui::storage::open_regular_file(&path)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Recovery draft disappeared"))
}
fn read_header(reader: &mut BufReader<File>) -> io::Result<Metadata> {
    let mut bytes = Vec::new();
    reader.take(65_537).read_until(b'\n', &mut bytes)?;
    if bytes.len() > 65_536 || bytes.last() != Some(&b'\n') {
        return Err(io::Error::other("Invalid recovery header"));
    }
    let meta: Metadata = serde_json::from_slice(&bytes)?;
    if meta.version != 1 {
        return Err(io::Error::other("Unsupported recovery version"));
    }
    Ok(meta)
}
pub struct Candidate {
    lease: Lease,
    pub meta: Metadata,
}
impl Candidate {
    pub fn label(&self) -> String {
        self.meta
            .path
            .as_deref()
            .map(decode_path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Untitled draft".into())
    }
    pub fn load(&self) -> io::Result<Rope> {
        let mut reader = BufReader::new(open_draft(&self.lease.dir)?);
        let meta = read_header(&mut reader)?;
        let remaining = reader
            .get_ref()
            .metadata()?
            .len()
            .saturating_sub(reader.stream_position()?);
        if remaining != meta.bytes as u64 {
            return Err(io::Error::other(
                "Recovery draft length mismatch; retained unchanged",
            ));
        }
        let text = Rope::from_reader(&mut reader)?;
        if meta != self.meta
            || text.len_bytes() != meta.bytes
            || checksum(&meta, &text)? != meta.checksum
        {
            return Err(io::Error::other(
                "Recovery draft failed integrity check; retained unchanged",
            ));
        }
        Ok(text)
    }
    pub fn restore(&self, buffer: &mut Buffer, text: Rope) {
        buffer.restore_recovery(text, self.meta.crlf, self.meta.bom);
    }
    pub fn discard(&self) -> io::Result<()> {
        self.lease.remove_draft()
    }
}
pub fn discover(path: Option<&Path>) -> io::Result<Option<Candidate>> {
    discover_in(&root()?, path)
}
fn discover_in(root: &Path, path: Option<&Path>) -> io::Result<Option<Candidate>> {
    let expected = path.map(identity).transpose()?.as_deref().map(encode_path);
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut latest: Option<Candidate> = None;
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir()
            || !entry.file_name().to_string_lossy().starts_with("session-")
        {
            continue;
        }
        let Ok(lease) = Lease::claim(entry.path()) else {
            continue;
        }; // active instances own their drafts
        let file = match open_draft(&lease.dir) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        let meta = read_header(&mut BufReader::new(file))?;
        if meta.path == expected && latest.as_ref().is_none_or(|c| meta.time > c.meta.time) {
            latest = Some(Candidate { lease, meta });
        }
    }
    Ok(latest)
}
#[derive(Clone)]
struct Snapshot {
    text: Rope,
    path: Option<PathBuf>,
    format: Option<String>,
    crlf: bool,
    bom: bool,
}
impl Snapshot {
    fn from_buffer(b: &Buffer, format: Option<&str>) -> Self {
        Self {
            text: b.text.clone(),
            path: b.path.clone(),
            format: format.map(str::to_owned),
            crlf: b.crlf,
            bom: b.bom,
        }
    }
}
fn checksum(meta: &Metadata, text: &Rope) -> io::Result<u32> {
    let mut header = meta.clone();
    header.checksum = 0;
    let mut hash = crc32fast::Hasher::new();
    hash.update(&serde_json::to_vec(&header)?);
    for chunk in text.chunks() {
        hash.update(chunk.as_bytes());
    }
    Ok(hash.finalize())
}
fn write_snapshot(lease: &Lease, snapshot: &Snapshot) -> io::Result<()> {
    write_snapshot_before_commit(lease, snapshot, || Ok(()))
}
fn write_snapshot_before_commit(
    lease: &Lease,
    snapshot: &Snapshot,
    before_commit: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let mut meta = Metadata {
        version: 1,
        path: snapshot
            .path
            .as_deref()
            .map(identity)
            .transpose()?
            .as_deref()
            .map(encode_path),
        format: snapshot.format.clone(),
        crlf: snapshot.crlf,
        bom: snapshot.bom,
        bytes: snapshot.text.len_bytes(),
        checksum: 0,
        time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .min(u64::MAX as u128) as u64,
    };
    meta.checksum = checksum(&meta, &snapshot.text)?;
    let mut temp = tempfile::NamedTempFile::new_in(&lease.dir)?;
    {
        let mut writer = io::BufWriter::new(temp.as_file_mut());
        serde_json::to_writer(&mut writer, &meta)?;
        writer.write_all(b"\n")?;
        for chunk in snapshot.text.chunks() {
            writer.write_all(chunk.as_bytes())?;
        }
        writer.flush()?;
    }
    temp.as_file().sync_all()?;
    before_commit()?;
    let target = lease.dir.join("draft");
    if tapp_ui::storage::open_regular_file(&target)?.is_some() {
        tapp_ui::storage::preserve_replacement_metadata(&target, temp.as_file())?;
        temp.as_file().sync_all()?;
        tapp_ui::storage::persist_existing(temp, &target)?;
    } else {
        temp.persist_noclobber(&target).map_err(|e| e.error)?;
    }
    sync_dir(&lease.dir)?;
    if let Some(parent) = lease.dir.parent() {
        sync_dir(parent)?;
    }
    Ok(())
}
enum Job {
    Write(Snapshot),
    Clear,
}
#[derive(Default)]
struct Queue {
    pending: Option<Job>,
    busy: bool,
    stop: bool,
    error: Option<String>,
}
struct Worker {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    done: mpsc::Receiver<()>,
}
impl Worker {
    fn start(
        root: PathBuf,
        mut lease: Option<Lease>,
        wake_main: Option<crate::events::Wake>,
    ) -> io::Result<Self> {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let q = queue.clone();
        let (done_tx, done) = mpsc::channel();
        thread::Builder::new()
            .name("editio-recovery".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                loop {
                    let job = {
                        let (lock, wake) = &*q;
                        let mut state = lock.lock().unwrap();
                        while state.pending.is_none() && !state.stop {
                            state = wake.wait(state).unwrap();
                        }
                        match state.pending.take() {
                            Some(job) => {
                                state.busy = true;
                                job
                            }
                            None => break,
                        }
                    };
                    let result = match job {
                        Job::Write(snapshot) => (|| {
                            if lease.is_none() {
                                lease = Some(Lease::create(&root)?);
                            }
                            write_snapshot(lease.as_ref().unwrap(), &snapshot)
                        })(),
                        Job::Clear => lease.as_ref().map_or(Ok(()), Lease::remove_draft),
                    };
                    let mut state = q.0.lock().unwrap();
                    state.error = result.err().map(|e| e.to_string());
                    state.busy = false;
                    drop(state);
                    if let Some(wake) = &wake_main {
                        wake.notify();
                    }
                }
                drop(lease);
                let _ = done_tx.send(());
            })?;
        Ok(Self { queue, done })
    }
    fn send(&self, job: Job) {
        let (lock, wake) = &*self.queue;
        lock.lock().unwrap().pending = Some(job);
        wake.notify_one();
    }
    fn take_error(&self) -> Option<String> {
        self.queue.0.lock().unwrap().error.take()
    }
    fn finish(&self) -> io::Result<()> {
        let (lock, wake) = &*self.queue;
        lock.lock().unwrap().stop = true;
        wake.notify_one();
        self.done
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| {
                io::Error::other("Recovery write did not finish before shutdown deadline")
            })?;
        if let Some(error) = self.take_error() {
            return Err(io::Error::other(error));
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let (lock, wake) = &*self.queue;
        lock.lock().unwrap().stop = true;
        wake.notify_one();
    }
}
pub struct Recovery {
    pub wake: Option<crate::events::Wake>,
    root: PathBuf,
    lease: Option<Lease>,
    worker: Option<Worker>,
    observed: Option<(u64, Option<PathBuf>, bool, bool)>,
    due: Option<Instant>,
    pub comparing: bool,
}
impl Recovery {
    pub fn next_wakeup(&self) -> Option<Duration> {
        self.due
            .map(|due| due.saturating_duration_since(Instant::now()))
    }
    pub fn busy(&self) -> bool {
        self.worker.as_ref().is_some_and(|worker| {
            let state = worker.queue.0.lock().unwrap();
            state.busy || state.pending.is_some()
        })
    }
    pub fn new(candidate: Option<Candidate>) -> io::Result<Self> {
        Ok(Self {
            wake: None,
            root: root()?,
            lease: candidate.map(|c| c.lease),
            worker: None,
            observed: None,
            due: None,
            comparing: false,
        })
    }
    fn send(&mut self, job: Job) -> io::Result<()> {
        if self.worker.is_none() {
            self.worker = Some(Worker::start(
                self.root.clone(),
                self.lease.take(),
                self.wake.clone(),
            )?);
        }
        self.worker.as_ref().unwrap().send(job);
        Ok(())
    }
    pub fn observe(&mut self, b: &Buffer, format: Option<&str>) -> io::Result<()> {
        let changed = self
            .observed
            .as_ref()
            .is_none_or(|(revision, path, crlf, bom)| {
                *revision != b.revision() || path != &b.path || *crlf != b.crlf || *bom != b.bom
            });
        if changed {
            self.observed = Some((b.revision(), b.path.clone(), b.crlf, b.bom));
            self.due = (b.dirty()
                || (b.path.is_none() && b.text.len_chars() > 0)
                || self.worker.is_some()
                || self.lease.is_some())
            .then(|| Instant::now() + DEBOUNCE);
        }
        if self.due.is_some_and(|due| Instant::now() >= due) {
            self.checkpoint(b, format)?;
        }
        if let Some(error) = self.worker.as_ref().and_then(Worker::take_error) {
            self.due = Some(Instant::now() + DEBOUNCE);
            return Err(io::Error::other(error));
        }
        Ok(())
    }
    pub fn checkpoint(&mut self, b: &Buffer, format: Option<&str>) -> io::Result<()> {
        self.due = None;
        let result = if b.dirty() || (b.path.is_none() && b.text.len_chars() > 0) {
            self.send(Job::Write(Snapshot::from_buffer(b, format)))
        } else {
            self.clear()
        };
        if result.is_err() {
            self.due = Some(Instant::now() + DEBOUNCE);
        }
        result
    }
    pub fn clear(&mut self) -> io::Result<()> {
        self.due = None;
        if self.worker.is_some() || self.lease.is_some() {
            self.send(Job::Clear)?;
        }
        Ok(())
    }
    pub fn finish(&mut self) -> io::Result<()> {
        self.worker.as_ref().map_or(Ok(()), Worker::finish)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(path: Option<PathBuf>, text: &str) -> Snapshot {
        Snapshot {
            text: Rope::from_str(text),
            path,
            format: None,
            crlf: false,
            bom: false,
        }
    }
    #[test]
    fn draft_roundtrip_detects_corruption_and_active_owner_is_skipped() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source.txt");
        fs::write(&path, "disk").unwrap();
        let lease = Lease::create(root.path()).unwrap();
        write_snapshot(&lease, &snapshot(Some(path.clone()), "draft 猫\n")).unwrap();
        assert!(discover_in(root.path(), Some(&path)).unwrap().is_none());
        drop(lease);
        let candidate = discover_in(root.path(), Some(&path)).unwrap().unwrap();
        assert_eq!(candidate.load().unwrap().to_string(), "draft 猫\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "disk");
        OpenOptions::new()
            .append(true)
            .open(candidate.lease.dir.join("draft"))
            .unwrap()
            .write_all(b"broken")
            .unwrap();
        assert!(candidate.load().is_err());
        assert!(candidate.lease.dir.join("draft").exists());
    }
    #[test]
    fn identical_basenames_in_different_directories_keep_separate_drafts() {
        let root = tempfile::tempdir().unwrap();
        let storage = root.path().join("recovery");
        let first = root.path().join("one/note.md");
        let second = root.path().join("two/note.md");
        for (path, content) in [(&first, "first"), (&second, "second")] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let lease = Lease::create(&storage).unwrap();
            write_snapshot(&lease, &snapshot(Some(path.clone()), content)).unwrap();
        }
        for (path, content) in [(&first, "first"), (&second, "second")] {
            let draft = discover_in(&storage, Some(path)).unwrap().unwrap();
            assert_eq!(draft.load().unwrap().to_string(), content);
            assert!(!path.exists());
        }
    }
    #[test]
    fn pending_clear_cannot_be_followed_by_an_older_write() {
        let root = tempfile::tempdir().unwrap();
        let worker = Worker::start(root.path().into(), None, None).unwrap();
        for _ in 0..50 {
            worker.send(Job::Write(snapshot(None, &"large ".repeat(100_000))));
        }
        worker.send(Job::Clear);
        worker.finish().unwrap();
        assert!(discover_in(root.path(), None).unwrap().is_none());
    }
    #[test]
    fn restore_keeps_current_disk_conflict_baseline_and_forces_explicit_save() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file");
        fs::write(&path, "disk\r\n").unwrap();
        let mut b = Buffer::open(&path).unwrap();
        b.restore_recovery(Rope::from_str("draft\n"), true, true);
        assert!(b.dirty());
        fs::write(&path, "external").unwrap();
        assert!(b.save().is_err());
        assert!(b.dirty());
        assert_eq!(fs::read_to_string(path).unwrap(), "external");
    }
    #[test]
    fn debounce_is_thirty_seconds_and_unchanged_observations_do_not_extend_it() {
        let root = tempfile::tempdir().unwrap();
        let mut r = Recovery {
            wake: None,
            root: root.path().into(),
            lease: None,
            worker: None,
            observed: None,
            due: None,
            comparing: false,
        };
        let mut b = Buffer::new("");
        b.insert("draft");
        r.observe(&b, None).unwrap();
        let due = r.due.unwrap();
        assert!(due.duration_since(Instant::now()) > Duration::from_secs(29));
        r.observe(&b, None).unwrap();
        assert_eq!(r.due, Some(due));
        assert!(r.worker.is_none());
        b.insert(" changed");
        r.observe(&b, None).unwrap();
        assert!(r.due.unwrap() >= due);
        r.due = Some(Instant::now());
        r.observe(&b, Some("rust")).unwrap();
        r.finish().unwrap();
        let candidate = discover_in(root.path(), None).unwrap().unwrap();
        assert_eq!(candidate.load().unwrap().to_string(), "draft changed");
        assert_eq!(candidate.meta.format.as_deref(), Some("rust"));
    }
    #[test]
    fn failed_replacement_preserves_previous_committed_checkpoint() {
        let root = tempfile::tempdir().unwrap();
        let lease = Lease::create(root.path()).unwrap();
        write_snapshot(&lease, &snapshot(None, "previous")).unwrap();
        let old = fs::read(lease.dir.join("draft")).unwrap();
        assert!(
            write_snapshot_before_commit(&lease, &snapshot(None, "new"), || Err(io::Error::other(
                "injected commit failure"
            )))
            .is_err()
        );
        assert_eq!(fs::read(lease.dir.join("draft")).unwrap(), old);
    }
    #[test]
    fn comparison_ends_after_accepting_recovered_text() {
        let mut current = Buffer::new("current\n");
        current.path = Some("document.md".into());
        let mut b = Buffer::new("current\n");
        b.restore_recovery(Rope::from_str("draft\n"), false, false);
        let mut e = editio::Editor::new(b);
        e.compare_with(&current).unwrap();
        assert!(e.is_comparison());
        e.end_comparison();
        assert!(!e.is_comparison());
        assert_eq!(e.buffer.text.to_string(), "draft\n");
    }
}

#[cfg(test)]
mod measurements {
    use super::*;
    #[test]
    #[ignore = "manual release performance measurement"]
    fn checkpoint_cost() {
        let root = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let lease = Lease::create(root.path()).unwrap();
        for size in [2 * 1024 * 1024, 16 * 1024 * 1024] {
            let text = Rope::from_str(&"x".repeat(size));
            let started = Instant::now();
            for _ in 0..10_000 {
                std::hint::black_box(text.clone());
            }
            let clones = started.elapsed();
            let snapshot = Snapshot {
                text,
                path: None,
                format: None,
                crlf: false,
                bom: false,
            };
            #[cfg(target_os = "linux")]
            let rss_before = std::fs::read_to_string("/proc/self/status")
                .unwrap()
                .lines()
                .find(|l| l.starts_with("VmRSS:"))
                .unwrap()
                .to_owned();
            let started = Instant::now();
            write_snapshot(&lease, &snapshot).unwrap();
            eprintln!(
                "bytes={size} clone_10000={clones:?} checkpoint={:?}",
                started.elapsed()
            );
            #[cfg(target_os = "linux")]
            eprintln!(
                "{rss_before} -> {}",
                std::fs::read_to_string("/proc/self/status")
                    .unwrap()
                    .lines()
                    .find(|l| l.starts_with("VmRSS:"))
                    .unwrap()
            );
        }
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    #[test]
    fn recovered_snapshot_preserves_bom_crlf_and_original_until_save() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("original");
        fs::write(&path, "\u{feff}original\r\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert("edited ");
        let lease = Lease::create(root.path()).unwrap();
        write_snapshot(&lease, &Snapshot::from_buffer(&buffer, None)).unwrap();
        drop(lease);
        let candidate = discover_in(root.path(), Some(&path)).unwrap().unwrap();
        let mut current = Buffer::open(&path).unwrap();
        candidate.restore(&mut current, candidate.load().unwrap());
        assert!(current.bom && current.crlf && current.dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "\u{feff}original\r\n");
        current.save().unwrap();
        assert!(!current.dirty());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "\u{feff}edited original\r\n"
        );
    }
    #[cfg(unix)]
    #[test]
    fn native_non_utf8_paths_roundtrip_without_loss() {
        use std::os::unix::ffi::OsStrExt;
        let path = Path::new(std::ffi::OsStr::from_bytes(b"/tmp/non-utf8-\xff"));
        assert_eq!(decode_path(&encode_path(path)), path);
    }
}

const RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const CLEANUP_QUIET: Duration = Duration::from_secs(2);
#[derive(Default)]
struct CleanupControl {
    finished: bool,
    idle: bool,
    stop: bool,
}
/// No scan, worker, or allocation at construction. One throttled pass after startup idle.
#[derive(Default)]
pub struct Cleanup {
    pub wake: Option<crate::events::Wake>,
    quiet_since: Option<Instant>,
    control: Option<Arc<(Mutex<CleanupControl>, Condvar)>>,
    attempted: bool,
}
impl Cleanup {
    pub fn next_wakeup(&self) -> Option<Duration> {
        if self.attempted
            && self
                .control
                .as_ref()
                .is_none_or(|c| c.0.lock().unwrap().idle)
        {
            return None;
        }
        self.quiet_since
            .map(|at| (at + CLEANUP_QUIET).saturating_duration_since(Instant::now()))
    }
    pub fn idle(&mut self, idle: bool) {
        if self.attempted && self.control.is_none() {
            return;
        }
        if self
            .control
            .as_ref()
            .is_some_and(|c| c.0.lock().unwrap().finished)
        {
            self.control = None;
            return;
        }
        if !idle {
            self.quiet_since = None;
        }
        let ready =
            idle && self.quiet_since.get_or_insert_with(Instant::now).elapsed() >= CLEANUP_QUIET;
        if ready && !self.attempted {
            self.attempted = true;
            let control = Arc::new((
                Mutex::new(CleanupControl {
                    finished: false,
                    idle: true,
                    stop: false,
                }),
                Condvar::new(),
            ));
            let worker = control.clone();
            let wake_main = self.wake.clone();
            if thread::Builder::new()
                .name("draft-retention".into())
                .stack_size(256 * 1024)
                .spawn(move || {
                    let work = || {
                        let wait = || {
                            let mut state = worker.0.lock().unwrap();
                            while !state.idle && !state.stop {
                                state = worker.1.wait(state).unwrap();
                            }
                            !state.stop
                        };
                        if !wait() {
                            return;
                        }
                        let Ok(root) = root() else {
                            return;
                        };
                        let Ok(entries) = fs::read_dir(root) else {
                            return;
                        };
                        let now = SystemTime::now();
                        for entry in entries {
                            if !wait() {
                                break;
                            }
                            if let Ok(entry) = entry
                                && entry.file_name().to_string_lossy().starts_with("session-")
                            {
                                let _ = evict_expired(&entry.path(), now);
                            }
                            // Yield disk/CPU bandwidth between individual bounded header reads.
                            thread::sleep(Duration::from_millis(10));
                        }
                    };
                    work();
                    worker.0.lock().unwrap().finished = true;
                    if let Some(wake) = wake_main {
                        wake.notify();
                    }
                })
                .is_ok()
            {
                self.control = Some(control);
            }
        }
        if let Some(control) = &self.control {
            let mut state = control.0.lock().unwrap();
            if state.idle != ready {
                state.idle = ready;
                if ready {
                    control.1.notify_one();
                }
            }
        }
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(control) = &self.control {
            control.0.lock().unwrap().stop = true;
            control.1.notify_one();
        }
    }
}
fn evict_expired(dir: &Path, now: SystemTime) -> io::Result<bool> {
    // Lock before inspecting; active/recovered sessions and symlinks are never removed.
    let lease = Lease::claim(dir.to_owned())?;
    let file = open_draft(&lease.dir)?;
    let modified = file.metadata()?.modified()?;
    let metadata = read_header(&mut BufReader::new(file))?;
    let written = UNIX_EPOCH + Duration::from_nanos(metadata.time);
    if now.duration_since(written).unwrap_or_default() <= RETENTION
        || now.duration_since(modified).unwrap_or_default() <= RETENTION
    {
        return Ok(false);
    }
    lease.remove_draft()?;
    Ok(true)
}

#[cfg(test)]
mod retention_tests {
    use super::*;
    #[test]
    fn eviction_requires_both_old_checkpoint_and_mtime_and_an_unlocked_lease() {
        let root = tempfile::tempdir().unwrap();
        let lease = Lease::create(root.path()).unwrap();
        write_snapshot(
            &lease,
            &Snapshot {
                text: Rope::from_str("draft"),
                path: None,
                format: None,
                crlf: false,
                bom: false,
            },
        )
        .unwrap();
        let dir = lease.dir.clone();
        let future = SystemTime::now() + RETENTION + Duration::from_secs(1);
        assert!(evict_expired(&dir, future).is_err());
        drop(lease);
        assert!(!evict_expired(&dir, SystemTime::now()).unwrap());
        let file = OpenOptions::new()
            .write(true)
            .open(dir.join("draft"))
            .unwrap();
        file.set_modified(future).unwrap();
        assert!(!evict_expired(&dir, future).unwrap());
        file.set_modified(SystemTime::now()).unwrap();
        drop(file);
        assert!(evict_expired(&dir, future).unwrap());
        assert!(!dir.exists());
    }
    #[test]
    fn busy_startup_never_starts_cleanup() {
        let mut cleanup = Cleanup::default();
        for _ in 0..100 {
            cleanup.idle(false);
        }
        assert!(!cleanup.attempted);
        assert!(cleanup.control.is_none());
        cleanup.idle(true);
        assert!(!cleanup.attempted);
        cleanup.idle(false);
        assert!(cleanup.quiet_since.is_none());
    }
}
