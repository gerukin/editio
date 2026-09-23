#![cfg(unix)]
//! Opt-in desktop test: requires a real clipboard session and briefly replaces its text.
use std::{
    fs::File,
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct App {
    child: Child,
    master: File,
}
impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl App {
    fn start(path: &std::path::Path, config: &std::path::Path, view: bool) -> Self {
        let (mut master, mut slave) = (-1, -1);
        let size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &size,
                )
            },
            0
        );
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        unsafe {
            libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }
        let binary = std::env::var_os("EDITIO_TEST_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_editio").into());
        let mut command = Command::new(binary);
        command
            .arg(path)
            .env("TERM", "xterm-256color")
            .env("XDG_CONFIG_HOME", config)
            .env("HOME", config)
            .env("TAPP_UI_THEME_FILE", config.join("theme"));
        if view {
            command.arg("--view");
        }
        let child = command
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave))
            .spawn()
            .unwrap();
        let mut app = Self { child, master };
        app.wait_for(if view { "VIEW" } else { "EDIT" });
        app
    }
    fn send(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }
    fn drain(&mut self) -> String {
        let mut out = Vec::new();
        let mut chunk = [0; 16384];
        while let Ok(n) = self.master.read(&mut chunk) {
            if n == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..n]);
        }
        String::from_utf8_lossy(&out).into_owned()
    }
    fn wait_for(&mut self, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut output = String::new();
        loop {
            output.push_str(&self.drain());
            if output.contains(expected) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "missing {expected} in PTY output"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
struct ClipboardRestore(arboard::Clipboard, Option<String>);
impl Drop for ClipboardRestore {
    fn drop(&mut self) {
        if let Some(text) = self.1.take() {
            let _ = self.0.set_text(text);
        }
    }
}
#[test]
#[ignore = "requires desktop clipboard; temporarily replaces and restores clipboard text"]
fn desktop_double_click_explicit_copy_and_paste() {
    let mut clipboard = arboard::Clipboard::new().expect("desktop clipboard");
    let old = clipboard.get_text().ok();
    let mut clipboard = ClipboardRestore(clipboard, old);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy.txt");
    for view in [false, true] {
        std::fs::write(&path, "hello world").unwrap();
        let mut app = App::start(&path, dir.path(), view);
        clipboard.0.set_text("selection sentinel").unwrap();
        app.send(b"\x1b[<0;2;1M\x1b[<0;2;1m\x1b[<0;2;1M\x1b[<0;2;1m");
        app.wait_for("Copied");
        assert_eq!(
            clipboard.0.get_text().unwrap(),
            "hello",
            "double-click clipboard in view={view}"
        );
        if !view {
            app.send(b"\x1b[1;5H"); // Ctrl+Home
            std::thread::sleep(Duration::from_millis(50));
            app.drain();
            clipboard.0.set_text("selection sentinel").unwrap();
            for _ in 0..5 {
                app.send(b"\x1b[1;2C");
            }
            std::thread::sleep(Duration::from_millis(300));
            assert_eq!(
                clipboard.0.get_text().unwrap(),
                "selection sentinel",
                "Shift selection must not copy"
            );
            // Dismiss the previous mouse-copy toast before checking explicit-copy feedback.
            app.send(b"\x03");
            std::thread::sleep(Duration::from_millis(100));
            assert_eq!(clipboard.0.get_text().unwrap(), "hello", "Ctrl+C clipboard");
            app.send(b"\x1b[1;5F\x16\x13"); // Ctrl+End, Ctrl+V, Ctrl+S
            let deadline = Instant::now() + Duration::from_secs(5);
            while std::fs::read_to_string(&path).unwrap() != "hello worldhello" {
                app.drain();
                assert!(
                    Instant::now() < deadline,
                    "pasted clipboard text was not saved"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
