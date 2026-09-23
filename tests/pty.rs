#![cfg(unix)]
use std::{
    fs::File,
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn actual_terminal_theme_toggle_quit_and_lifecycle_restore() {
    terminal_lifecycle(None);
}

#[test]
fn selected_rows_follow_light_and_dark_terminal_backgrounds() {
    terminal_lifecycle(Some(("ffff/ffff/ffff", "48;2;239;239;239")));
    terminal_lifecycle(Some(("1a1a/1b1b/2626", "48;2;32;33;44")));
}

fn terminal_lifecycle(palette: Option<(&str, &str)>) {
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
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
                std::ptr::null_mut(),
                &raw mut size,
            )
        },
        0
    );
    let mut master = unsafe { File::from_raw_fd(master) };
    let slave = unsafe { File::from_raw_fd(slave) };
    let mut before = unsafe { std::mem::zeroed::<libc::termios>() };
    assert_eq!(
        unsafe { libc::tcgetattr(slave.as_raw_fd(), &mut before) },
        0
    );
    unsafe {
        libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
    }
    let dir = tempfile::tempdir().unwrap();
    let theme = dir.path().join(".config/editio/config.json");
    let binary = std::env::var_os("EDITIO_TEST_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_editio").into());
    let mut command = Command::new(binary);
    // Give the child its own controlling terminal for the startup OSC query.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = command
        .arg("-v")
        .env("TERM", "xterm-256color")
        .env_remove("NO_COLOR")
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("old-config"))
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()))
        .spawn()
        .unwrap();
    let mut child = Process(child);
    let mut output = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut phase = 0;
    let mut answered = false;
    loop {
        let mut bytes = [0; 16384];
        if let Ok(n) = master.read(&mut bytes) {
            output.extend_from_slice(&bytes[..n]);
        }
        let text = String::from_utf8_lossy(&output);
        if let Some((rgb, _)) = palette
            && !answered
            && text.contains("\x1b]11;?")
        {
            master
                .write_all(format!("\x1b]11;rgb:{rgb}\x1b\\\x1b[?1;2c").as_bytes())
                .unwrap();
            answered = true;
        }
        if phase == 0 && text.contains("VIEW") {
            #[cfg(target_os = "linux")]
            {
                let ticks = |pid| {
                    std::fs::read_to_string(format!("/proc/{pid}/stat"))
                        .ok()
                        .and_then(|s| {
                            let fields: Vec<_> = s.rsplit_once(')')?.1.split_whitespace().collect();
                            Some(
                                fields.get(11)?.parse::<u64>().ok()?
                                    + fields.get(12)?.parse::<u64>().ok()?,
                            )
                        })
                };
                let before = ticks(child.0.id());
                std::thread::sleep(Duration::from_millis(250));
                if let (Some(before), Some(after)) = (before, ticks(child.0.id())) {
                    eprintln!(
                        "Idle CPU over 250 ms: {} process ticks",
                        after.saturating_sub(before)
                    );
                }
            }
            master.write_all(b"\x0bToggle theme").unwrap();
            phase = 1;
        } else if phase == 1 && text.contains("Toggle theme") {
            if let Some((_, background)) = palette {
                assert!(
                    answered && text.contains(background),
                    "selection surface: {text:?}"
                );
                assert!(!text.contains('›'), "no selection chevron");
            }
            master.write_all(b"\r").unwrap();
            phase = 2;
        } else if phase == 2
            && std::fs::read_to_string(&theme)
                .ok()
                .is_some_and(|s| s.contains("tokyo-night-omarchy"))
        {
            master.write_all(b"\x11").unwrap();
            phase = 3;
        }
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "{text}");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "PTY timed out in phase {phase}: {text}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut bytes = [0; 16384];
    if let Ok(n) = master.read(&mut bytes) {
        output.extend_from_slice(&bytes[..n]);
    }
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("?1049h") && text.contains("?1049l"),
        "alternate screen restored"
    );
    assert!(
        text.contains("38;2;") || text.contains("48;2;"),
        "fixed palette rendered as RGB: {text:?}"
    );
    let mut after = unsafe { std::mem::zeroed::<libc::termios>() };
    assert_eq!(unsafe { libc::tcgetattr(slave.as_raw_fd(), &mut after) }, 0);
    assert_eq!(
        before.c_lflag & (libc::ICANON | libc::ECHO),
        after.c_lflag & (libc::ICANON | libc::ECHO)
    );
}
