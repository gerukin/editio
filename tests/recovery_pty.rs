#![cfg(unix)]
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct App {
    child: Child,
    tty: File,
    output: String,
    errors: File,
}
impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl App {
    fn start(home: &Path, file: Option<&Path>) -> Self {
        // openpty cannot atomically set CLOEXEC. Prevent another test's fork
        // from inheriting a master in the interval before fcntl, which would
        // conceal terminal loss until that unrelated child exits.
        static START: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _start = START.lock().unwrap_or_else(|e| e.into_inner());
        let (mut master, mut slave) = (-1, -1);
        let mut size = libc::winsize {
            ws_row: 30,
            ws_col: 100,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut size,
                )
            },
            0
        );
        let tty = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        unsafe {
            libc::fcntl(tty.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
            // The child must not retain the terminal emulator's master endpoint.
            libc::fcntl(tty.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(slave.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC);
        }
        let mut command = Command::new(
            std::env::var_os("EDITIO_TEST_BINARY")
                .unwrap_or_else(|| env!("CARGO_BIN_EXE_editio").into()),
        );
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        if let Some(file) = file {
            command.arg(file);
        }
        let errors = tempfile::tempfile().unwrap();
        let child = command
            .current_dir(home)
            .env("HOME", home)
            .env("TERM", "xterm-256color")
            .env("NO_COLOR", "1")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(errors.try_clone().unwrap()))
            .spawn()
            .unwrap();
        Self {
            child,
            tty,
            output: String::new(),
            errors,
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.output.clear();
        self.tty.write_all(bytes).unwrap();
    }
    fn wait(&mut self, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.read();
            if self.output.contains(needle) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "waiting for {needle}: {}",
                self.output
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn read(&mut self) {
        let mut bytes = [0; 65536];
        while let Ok(n) = self.tty.read(&mut bytes) {
            if n == 0 {
                break;
            }
            self.output.push_str(&String::from_utf8_lossy(&bytes[..n]));
        }
    }
    fn signal(&mut self, signal: i32) {
        assert_eq!(unsafe { libc::kill(self.child.id() as i32, signal) }, 0);
    }
    fn exit(&mut self, code: i32) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.read();
            if let Some(status) = self.child.try_wait().unwrap() {
                use std::io::{Seek, SeekFrom};
                self.errors.seek(SeekFrom::Start(0)).unwrap();
                let mut errors = String::new();
                self.errors.read_to_string(&mut errors).unwrap();
                assert_eq!(status.code(), Some(code), "{}\n{errors}", self.output);
                return;
            }
            assert!(Instant::now() < deadline, "{}", self.output);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
fn drafts(home: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(home.join(".local/state/editio/recovery"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path().join("draft"))
        .filter(|p| p.exists())
        .collect()
}
fn make_draft(home: &Path, file: Option<&Path>, signal: i32) {
    let mut app = App::start(home, file);
    app.wait("EDIT");
    app.send(b"\x1b[200~RECOVERME\x1b[201~");
    app.wait("ECOVERME");
    app.signal(signal);
    app.exit(1);
    assert_eq!(drafts(home).len(), 1);
}
#[test]
fn sigterm_recovers_against_current_disk_then_save_accepts_and_cleans() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("note.txt");
    fs::write(&file, "original\n").unwrap();
    make_draft(dir.path(), Some(&file), libc::SIGTERM);
    assert_eq!(fs::read_to_string(&file).unwrap(), "original\n");
    fs::write(&file, "external current\n").unwrap();
    let mut app = App::start(dir.path(), Some(&file));
    app.wait("Recovery draft found");
    app.send(b"\r");
    app.wait("ECOVERME");
    app.send(b"[");
    app.wait("external"); // previous version is the fresh disk contents
    app.send(b"]\x13");
    app.wait("Saved");
    app.send(b"\x11");
    app.exit(0);
    assert_eq!(fs::read_to_string(&file).unwrap(), "RECOVERMEoriginal\n");
    assert!(drafts(dir.path()).is_empty());
}
#[test]
fn sighup_untitled_recovery_and_explicit_disk_discard() {
    let dir = tempfile::tempdir().unwrap();
    make_draft(dir.path(), None, libc::SIGHUP);
    let mut app = App::start(dir.path(), None);
    app.wait("Recovery draft found");
    app.send(b"\x1b[D\r");
    app.wait("EDIT");
    app.send(b"\x11");
    app.exit(0);
    assert!(drafts(dir.path()).is_empty());
}
#[test]
fn leaving_recovery_prompt_preserves_draft() {
    let dir = tempfile::tempdir().unwrap();
    make_draft(dir.path(), None, libc::SIGTERM);
    let before = fs::read(&drafts(dir.path())[0]).unwrap();
    let mut app = App::start(dir.path(), None);
    app.wait("Recovery draft found");
    app.send(b"\x1b");
    app.exit(1);
    assert_eq!(fs::read(&drafts(dir.path())[0]).unwrap(), before);
}
#[test]
fn failed_save_during_recovery_keeps_draft_and_external_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("note.txt");
    fs::write(&file, "original").unwrap();
    make_draft(dir.path(), Some(&file), libc::SIGTERM);
    let mut app = App::start(dir.path(), Some(&file));
    app.wait("Recovery draft found");
    app.send(b"\r");
    app.wait("ECOVERME");
    fs::write(&file, "external after opening").unwrap();
    app.send(b"\x13");
    app.wait("cancelled");
    app.signal(libc::SIGTERM);
    app.exit(1);
    assert_eq!(fs::read_to_string(file).unwrap(), "external after opening");
    assert_eq!(drafts(dir.path()).len(), 1);
}
#[test]
fn actual_thirty_second_debounce_checkpoints_without_exit() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::start(dir.path(), None);
    app.wait("EDIT");
    app.send(b"\x1b[200~RECOVERME\x1b[201~");
    app.wait("ECOVERME");
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(28) {
        assert!(drafts(dir.path()).is_empty());
        app.read();
        std::thread::sleep(Duration::from_millis(100));
    }
    while drafts(dir.path()).is_empty() {
        assert!(started.elapsed() < Duration::from_secs(36));
        app.read();
        std::thread::sleep(Duration::from_millis(100));
    }
    #[cfg(target_os = "linux")]
    {
        std::thread::sleep(Duration::from_secs(3));
        let draft = drafts(dir.path()).pop().unwrap();
        let modified = fs::metadata(&draft).unwrap().modified().unwrap();
        idle_sample(&mut app, "draft already checkpointed");
        assert_eq!(fs::metadata(draft).unwrap().modified().unwrap(), modified);
    }
    app.signal(libc::SIGKILL);
    let _ = app.child.wait();
    assert_eq!(drafts(dir.path()).len(), 1);
}

#[test]
fn relative_named_file_save_reopen_checkpoint_and_recover() {
    let dir = tempfile::tempdir().unwrap();
    let relative = Path::new("name.md");
    fs::write(dir.path().join(relative), "original\n").unwrap();
    let mut app = App::start(dir.path(), Some(relative));
    app.wait("EDIT");
    app.send(b"first");
    app.send(b"\x13");
    app.wait("Saved");
    app.send(b"\x11");
    app.exit(0);
    let saved = fs::read(dir.path().join(relative)).unwrap();
    let mut app = App::start(dir.path(), Some(relative));
    app.wait("EDIT");
    app.send(b"\x1b[200~RECOVERME\x1b[201~");
    app.wait("ECOVERME");
    let started = Instant::now();
    while drafts(dir.path()).is_empty() {
        assert!(started.elapsed() < Duration::from_secs(36));
        app.read();
        std::thread::sleep(Duration::from_millis(100));
    }
    app.signal(libc::SIGKILL);
    app.child.wait().unwrap();
    assert_eq!(fs::read(dir.path().join(relative)).unwrap(), saved);
    let mut reopened = App::start(dir.path(), Some(relative));
    reopened.wait("Recovery draft found");
    reopened.send(b"\r");
    reopened.wait("ECOVERME");
}

#[test]
fn never_saved_named_file_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let file = Path::new("not-created.md");
    make_draft(dir.path(), Some(file), libc::SIGHUP);
    assert!(!dir.path().join(file).exists());
    let mut app = App::start(dir.path(), Some(file));
    app.wait("Recovery draft found");
    app.send(b"\r");
    app.wait("ECOVERME");
    assert!(!dir.path().join(file).exists());
}

#[test]
fn closing_terminal_exits_and_releases_recovery_lock() {
    let dir = tempfile::tempdir().unwrap();
    let file = Path::new("closed-terminal.md");
    let mut app = App::start(dir.path(), Some(file));
    app.wait("EDIT");
    app.send(b"\x1b[200~RECOVERME\x1b[201~");
    app.wait("ECOVERME");
    // Closing the PTY master is terminal loss, not merely an injected signal.
    app.tty = File::open("/dev/null").unwrap();
    app.exit(1);
    assert_eq!(drafts(dir.path()).len(), 1);
    let mut reopened = App::start(dir.path(), Some(file));
    reopened.wait("Recovery draft found");
    reopened.send(b"\x1b");
    reopened.exit(1);
}

#[test]
fn real_terminal_navigator_reveals_hidden_heading_without_editing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("navigation.md");
    let original = "# Top\n\n<details>\n<summary>Closed</summary>\n\n## Secret destination\n\nHere.\n\n</details>\n";
    fs::write(&file, original).unwrap();
    let mut app = App::start(dir.path(), Some(&file));
    app.wait("EDIT");
    app.send(b"\x07");
    app.wait("VIEW");
    app.send(b"\x14");
    app.wait("Headings");
    app.send(b"secret");
    app.wait("Secret");
    app.send(b"\r");
    app.wait("destination");
    app.send(b"\x11");
    app.exit(0);
    assert_eq!(fs::read_to_string(file).unwrap(), original);
}

#[cfg(target_os = "linux")]
fn idle_sample(app: &mut App, label: &str) {
    fn counters(pid: u32) -> (u64, u64) {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let fields: Vec<_> = stat
            .rsplit_once(')')
            .unwrap()
            .1
            .split_whitespace()
            .collect();
        let ticks = fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap();
        let switches = fs::read_dir(format!("/proc/{pid}/task"))
            .unwrap()
            .flatten()
            .filter_map(|entry| fs::read_to_string(entry.path().join("status")).ok())
            .flat_map(|status| {
                status
                    .lines()
                    .filter(|line| line.starts_with("voluntary_ctxt_switches:"))
                    .map(|line| {
                        line.split_whitespace()
                            .last()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
            })
            .sum();
        (ticks, switches)
    }
    app.read();
    let output = app.output.len();
    let before = counters(app.child.id());
    std::thread::sleep(Duration::from_secs(2));
    let after = counters(app.child.id());
    app.read();
    eprintln!(
        "{label}: 2s idle: {} CPU ticks, {} voluntary thread switches, {} output bytes",
        after.0 - before.0,
        after.1.saturating_sub(before.1),
        app.output.len() - output
    );
    assert!(after.0 - before.0 <= 1, "idle CPU busy loop: {label}");
    assert!(
        after.1.saturating_sub(before.1) <= 2,
        "idle polling: {label}"
    );
    assert_eq!(app.output.len(), output, "idle redraw: {label}");
}

#[test]
#[cfg(target_os = "linux")]
fn idle_clean_dirty_and_large_documents_sleep_without_polling() {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in [
        ("small.md", "# Quiet\n\nText\n".to_owned()),
        ("large.rs", "// idle source\n".repeat(250_000)),
    ] {
        let file = dir.path().join(name);
        fs::write(&file, text).unwrap();
        let mut app = App::start(dir.path(), Some(&file));
        app.wait("EDIT");
        std::thread::sleep(Duration::from_secs(4));
        idle_sample(&mut app, name);
        app.send(b"x");
        std::thread::sleep(Duration::from_secs(4));
        idle_sample(&mut app, "dirty, debounce pending");
        app.send(b"\x07");
        app.wait("VIEW");
        std::thread::sleep(Duration::from_secs(4));
        idle_sample(&mut app, "view");
        app.signal(libc::SIGTERM);
        app.exit(1);
    }
}

#[test]
fn terminal_loss_exits_in_view_and_modals_and_repeated_clean_quit_exits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lifecycle.md");
    fs::write(&path, "# Lifecycle\n\nClean file.\n").unwrap();
    for (keys, title) in [
        (b"\x07".as_slice(), "VIEW"),
        (b"\x0b", "Command center"),
        (b"\x06", "Find"),
        (b"\x07i", "Info"),
        (b"\x07?", "Help"),
    ] {
        let mut app = App::start(dir.path(), Some(&path));
        app.wait("EDIT");
        app.send(keys);
        app.wait(title);
        app.tty = File::open("/dev/null").unwrap();
        app.exit(1);
    }
    for _ in 0..8 {
        let mut app = App::start(dir.path(), Some(&path));
        app.wait("EDIT");
        app.send(b"\x11");
        app.exit(0);
    }
    assert!(drafts(dir.path()).is_empty());
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        "# Lifecycle\n\nClean file.\n"
    );
}

#[test]
fn terminal_loss_during_dirty_view_preparation_keeps_recoverable_draft() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("busy.md");
    fs::write(&file, "# Work\n\nSome **rendered** text.\n\n".repeat(4_000)).unwrap();
    let mut app = App::start(dir.path(), Some(&file));
    app.wait("EDIT");
    app.send(b"RECOVERME\x07");
    app.wait("VIEW");
    app.tty = File::open("/dev/null").unwrap();
    app.exit(1);
    assert_eq!(drafts(dir.path()).len(), 1);
    let mut reopened = App::start(dir.path(), Some(&file));
    reopened.wait("Recovery draft found");
    reopened.tty = File::open("/dev/null").unwrap();
    reopened.exit(1);
    assert_eq!(drafts(dir.path()).len(), 1);
}

#[test]
fn cancelling_quit_does_not_arm_the_shutdown_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cancel.txt");
    fs::write(&path, "original").unwrap();
    let mut app = App::start(dir.path(), Some(&path));
    app.wait("EDIT");
    app.send(b"unsaved\x11");
    app.wait("Unsaved changes");
    app.send(b"\x1b");
    std::thread::sleep(Duration::from_secs(6));
    assert!(
        app.child.try_wait().unwrap().is_none(),
        "cancelled quit must keep editing"
    );
    app.send(b"\x11");
    app.wait("Unsaved changes");
    app.send(b"d");
    app.exit(0);
    assert_eq!(fs::read_to_string(path).unwrap(), "original");
}

#[test]
fn file_finder_prepopulates_tag_and_switches_with_save_cancel_and_history() {
    let home = tempfile::tempdir().unwrap();
    let first = home.path().join("first.md");
    let second = home.path().join("second.md");
    fs::write(&first, "FIRST ORIGINAL\n").unwrap();
    fs::write(&second, "SECOND DESTINATION\n").unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_editio"))
        .env("HOME", home.path())
        .args(["--tag", "fixture"])
        .arg(&second)
        .status()
        .unwrap();
    assert!(status.success());
    let mut app = App::start(home.path(), Some(&first));
    app.wait("ORIGINAL");
    app.send(b"\x1b[200~changed \x1b[201~");
    app.wait("changed");
    app.send(b"\x0bfind other files\r");
    app.wait("Find");
    app.send(b"\x0btag: fixture\r");
    app.wait("second.md");
    app.send(b"\r");
    app.wait("Unsaved");
    app.send(b"\x1b");
    app.wait("?25h");
    assert_eq!(fs::read_to_string(&first).unwrap(), "FIRST ORIGINAL\n");
    app.send(b"\x0bfind other files\r");
    app.wait("second.md");
    app.send(b"\r");
    app.wait("Unsaved");
    app.send(b"s");
    app.wait("DESTINATION");
    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "changed FIRST ORIGINAL\n"
    );
    app.send(b"\x11");
    app.exit(0);
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(home.path().join(".config/editio/config.json")).unwrap())
            .unwrap();
    assert!(
        config["navigation"]["viewed"]
            .get(second.to_str().unwrap())
            .is_some()
    );
    assert!(
        config["navigation"]["modified"]
            .get(first.to_str().unwrap())
            .is_some()
    );
}

#[test]
fn find_cycles_scopes_and_uses_one_contextual_command_view() {
    let home = tempfile::tempdir().unwrap();
    let file = home.path().join("paperclip.md");
    fs::write(&file, "A paperclip found its calling.\n").unwrap();
    let mut app = App::start(home.path(), Some(&file));
    app.wait("EDIT");
    app.send(b"\x06paperclip\x0b");
    app.wait("fuzzy");
    app.send(b"use fuzzy\r\t");
    app.wait("Recently viewed");
    app.send(b"\x0b");
    app.wait("exact");
    app.send(b"\x0b\x1b[Z\x0b");
    app.wait("exact");
    app.send(b"\x0b\r\x11");
    app.exit(0);
    assert_eq!(
        fs::read_to_string(file).unwrap(),
        "A paperclip found its calling.\n"
    );
}
