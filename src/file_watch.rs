//! Event-driven, single-document monitoring. No idle timer or filesystem polling.
use crate::events::Wake;
use editio::buffer::{Buffer, DiskBaseline};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tapp_ui::background::Latest;

const DEBOUNCE: Duration = Duration::from_millis(150);
const MAX_RELOAD: usize = 64 * 1024 * 1024;
struct Request {
    generation: u64,
    path: PathBuf,
    baseline: DiskBaseline,
}
struct Completion {
    generation: u64,
    result: io::Result<Option<Buffer>>,
}
pub struct Monitor {
    enabled: bool,
    path: Option<PathBuf>,
    target: Option<PathBuf>,
    watcher: Option<RecommendedWatcher>,
    changed: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    deadline: Option<Instant>,
    generation: u64,
    worker: Option<Latest<Request>>,
    result: Arc<Mutex<Option<Completion>>>,
    wake: Wake,
}
impl Monitor {
    pub fn new(enabled: bool, wake: Wake) -> Self {
        Self {
            enabled,
            path: None,
            target: None,
            watcher: None,
            changed: Arc::default(),
            error: Arc::default(),
            deadline: None,
            generation: 0,
            worker: None,
            result: Arc::default(),
            wake,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.reset();
    }
    fn reset(&mut self) {
        self.watcher = None;
        self.path = None;
        self.target = None;
        self.deadline = None;
        self.generation += 1;
        self.changed = Arc::default();
        self.error = Arc::default();
        if let Some(worker) = &self.worker {
            worker.cancel_pending();
        }
        self.result.lock().unwrap().take();
    }
    /// Call on document transitions, not as a filesystem poll.
    pub fn sync(&mut self, buffer: &Buffer) -> io::Result<()> {
        let path = self.enabled.then_some(buffer.path.as_ref()).flatten();
        if path == self.path.as_ref()
            && (!self.enabled || self.target.as_deref() == buffer.disk_target())
        {
            return Ok(());
        }
        self.reset();
        let Some(path) = path else {
            return Ok(());
        };
        // Remember even failed registrations; do not retry on every keystroke.
        self.path = Some(path.clone());
        self.target = buffer.disk_target().map(Path::to_owned);
        let absolute = std::path::absolute(path)?;
        let parent = absolute
            .parent()
            .ok_or_else(|| io::Error::other("Missing parent directory"))?;
        if !parent.is_dir() && buffer.needs_creation() {
            return Ok(());
        }
        let target = std::fs::canonicalize(path).ok();
        let changed = self.changed.clone();
        let errors = self.error.clone();
        let wake = self.wake.clone();
        let filter = absolute.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
                Ok(event)
                    if !matches!(event.kind, EventKind::Access(_))
                        && (event.need_rescan()
                            || event
                                .paths
                                .iter()
                                .any(|p| p == &filter || Some(p) == target.as_ref())) =>
                {
                    if !changed.swap(true, Ordering::AcqRel) {
                        wake.notify();
                    }
                }
                Err(error) => {
                    *errors.lock().unwrap() = Some(error.to_string());
                    wake.notify();
                }
                _ => (),
            })
            .map_err(io::Error::other)?;
        watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
        // A symlink's target may be in another directory; still filter for this file only.
        if let Ok(target) = std::fs::canonicalize(path)
            && let Some(target_parent) = target.parent()
            && target_parent != parent
        {
            watcher
                .watch(target_parent, RecursiveMode::NonRecursive)
                .map_err(io::Error::other)?;
        }
        self.watcher = Some(watcher);
        // Covers the open-to-watch registration race, once per document.
        self.deadline = (!buffer.needs_creation()).then(|| Instant::now() + DEBOUNCE);
        Ok(())
    }
    /// Invalidate pre-save reads, rebind atomic replacements/symlinks and recheck once.
    pub fn saved(&mut self, buffer: &Buffer) -> io::Result<()> {
        self.reset();
        self.sync(buffer)
    }
    pub fn next_wakeup(&self) -> Option<Duration> {
        self.deadline
            .map(|d| d.saturating_duration_since(Instant::now()))
    }
    pub fn poll(&mut self, buffer: &Buffer) -> Option<io::Result<Buffer>> {
        if let Some(error) = self.error.lock().unwrap().take() {
            return Some(Err(io::Error::other(error)));
        }
        if self.changed.swap(false, Ordering::AcqRel) {
            self.generation += 1;
            self.deadline = Some(Instant::now() + DEBOUNCE);
        }
        if self.deadline.is_some_and(|d| Instant::now() >= d) {
            self.deadline = None;
            if let Some(path) = &self.path {
                let slot = self.result.clone();
                let wake = self.wake.clone();
                let worker = self.worker.get_or_insert_with(|| {
                    Latest::spawn(move |request: Request| {
                        let result = read_stable(&request.path, &request.baseline);
                        *slot.lock().unwrap() = Some(Completion {
                            generation: request.generation,
                            result,
                        });
                        wake.notify();
                    })
                });
                worker.submit(Request {
                    generation: self.generation,
                    path: path.clone(),
                    baseline: buffer.disk_baseline(),
                });
            }
        }
        let completion = self.result.lock().unwrap().take()?;
        if completion.generation != self.generation {
            return None;
        }
        match completion.result {
            Ok(buffer) => buffer.map(Ok),
            Err(error) => Some(Err(error)),
        }
    }
}
fn read_stable(path: &Path, baseline: &DiskBaseline) -> io::Result<Option<Buffer>> {
    let before = std::fs::metadata(path)?;
    if !before.is_file() {
        return Err(io::Error::other(
            "Not a regular file; current text retained",
        ));
    }
    let loaded = Buffer::read_changed(path, baseline, MAX_RELOAD)?;
    let after = std::fs::metadata(path)?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(io::Error::other(
            "File changed while reading; current text retained",
        ));
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wait(m: &mut Monitor, b: &Buffer) -> io::Result<Buffer> {
        let end = Instant::now() + Duration::from_secs(4);
        while Instant::now() < end {
            if let Some(result) = m.poll(b) {
                return result;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("No filesystem update");
    }
    fn settle(m: &mut Monitor, b: &Buffer) {
        for _ in 0..40 {
            assert!(m.poll(b).is_none());
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(m.next_wakeup().is_none());
    }
    #[test]
    fn native_events_reload_replacements_ignore_own_saves_and_release_watchers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.md");
        std::fs::write(&path, "first").unwrap();
        let mut b = Buffer::open(&path).unwrap();
        let mut m = Monitor::new(true, crate::events::test_wake());
        m.sync(&b).unwrap();
        settle(&mut m, &b);
        assert!(m.watcher.is_some());
        std::fs::write(&path, "second").unwrap();
        b = wait(&mut m, &b).unwrap();
        assert_eq!(b.text.to_string(), "second");
        b.insert("own ");
        b.save().unwrap();
        m.saved(&b).unwrap();
        settle(&mut m, &b);
        let replacement = dir.path().join("replacement");
        std::fs::write(&replacement, "atomic replacement").unwrap();
        #[cfg(windows)]
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(replacement, &path).unwrap();
        b = wait(&mut m, &b).unwrap();
        assert_eq!(b.text.to_string(), "atomic replacement");
        m.toggle();
        assert!(m.watcher.is_none());
        assert!(m.next_wakeup().is_none());
        std::fs::write(&path, "disabled").unwrap();
        settle(&mut m, &b);
        m.toggle();
        m.sync(&b).unwrap();
        assert_eq!(wait(&mut m, &b).unwrap().text.to_string(), "disabled");
        m.sync(&Buffer::new("untitled")).unwrap();
        assert!(m.watcher.is_none());
    }
    #[test]
    fn deletion_invalid_utf8_and_directory_do_not_become_empty_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.md");
        std::fs::write(&path, "keep").unwrap();
        let b = Buffer::open(&path).unwrap();
        let baseline = b.disk_baseline();
        std::fs::remove_file(&path).unwrap();
        assert!(read_stable(&path, &baseline).is_err());
        std::fs::write(&path, [0xff]).unwrap();
        assert!(read_stable(&path, &baseline).is_err());
        assert_eq!(b.text.to_string(), "keep");
        assert!(read_stable(dir.path(), &baseline).is_err());
    }
    #[test]
    fn unrelated_directory_events_do_not_schedule_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current");
        std::fs::write(&path, "keep").unwrap();
        let b = Buffer::open(&path).unwrap();
        let mut m = Monitor::new(true, crate::events::test_wake());
        m.sync(&b).unwrap();
        settle(&mut m, &b);
        for i in 0..100 {
            std::fs::write(dir.path().join(format!("other{i}")), "irrelevant").unwrap();
        }
        std::thread::sleep(Duration::from_millis(100));
        assert!(!m.changed.load(Ordering::Relaxed));
        assert!(m.poll(&b).is_none());
        assert!(m.next_wakeup().is_none());
    }
    #[test]
    #[cfg(unix)]
    fn retargeted_symlink_rebinds_without_watching_old_documents() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("one/file.txt");
        let second = dir.path().join("two/file.txt");
        for path in [&first, &second] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "initial").unwrap();
        }
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&first, &link).unwrap();
        let mut b = Buffer::open(&link).unwrap();
        let mut m = Monitor::new(true, crate::events::test_wake());
        m.sync(&b).unwrap();
        settle(&mut m, &b);
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&second, &link).unwrap();
        b = wait(&mut m, &b).unwrap();
        m.sync(&b).unwrap();
        settle(&mut m, &b);
        std::fs::write(&second, "new target update").unwrap();
        assert_eq!(
            wait(&mut m, &b).unwrap().text.to_string(),
            "new target update"
        );
        std::fs::write(first, "old target ignored").unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert!(!m.changed.load(Ordering::Relaxed));
    }
}
