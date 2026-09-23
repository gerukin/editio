use std::process::{Command, Stdio};

#[test]
fn nonterminal_invocation_fails_without_terminal_escape_sequences() {
    let output = Command::new(env!("CARGO_BIN_EXE_editio"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.contains(&0x1b));
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires terminal"));
    let help = Command::new(env!("CARGO_BIN_EXE_editio"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let invalid = Command::new(env!("CARGO_BIN_EXE_editio"))
        .arg("--invalid-option")
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
}

#[cfg(unix)]
mod pty {
    use super::*;
    use std::{
        fs::File,
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::process::CommandExt,
        },
        time::{Duration, Instant},
    };
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    // A synthetic caller: supplies a document, waits synchronously, and accepts
    // it only on status zero. No shell settings or real caller repositories used.
    fn run(directory: bool, steps: &[(&str, &[u8])], expected_code: i32, expected: &str) {
        run_input(directory, steps, expected_code, expected, None, &[], false);
    }

    fn run_input(
        directory: bool,
        steps: &[(&str, &[u8])],
        expected_code: i32,
        expected: &str,
        input: Option<&[u8]>,
        args: &[&str],
        keep_open: bool,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("-draft 日本語.txt");
        std::fs::write(&path, "Original message\n").unwrap();
        let comparison = args.first() == Some(&"--diff");
        if comparison {
            std::fs::write(dir.path().join("before.txt"), "Reference message\n").unwrap();
        }
        let mut master = -1;
        let mut slave = -1;
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
        let mut master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        unsafe {
            libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }
        let mut before = unsafe { std::mem::zeroed::<libc::termios>() };
        assert_eq!(
            unsafe { libc::tcgetattr(slave.as_raw_fd(), &mut before) },
            0
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_editio"));
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(1, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        if input.is_none() && args.is_empty() {
            command.arg("--").arg(if directory {
                std::ffi::OsStr::new(".")
            } else {
                path.file_name().unwrap()
            });
        } else {
            command.args(args);
        }
        let mut child = Child(
            command
                .current_dir(dir.path())
                .env("HOME", dir.path())
                .env("TERM", "xterm-256color")
                .env("NO_COLOR", "1")
                .stdin(if input.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::from(slave.try_clone().unwrap())
                })
                .stdout(Stdio::from(slave.try_clone().unwrap()))
                .stderr(Stdio::from(slave.try_clone().unwrap()))
                .spawn()
                .unwrap(),
        );
        let mut input_writer = child.0.stdin.take();
        if let Some(input) = input {
            input_writer.as_mut().unwrap().write_all(input).unwrap();
        }
        if !keep_open {
            drop(input_writer.take());
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut output = Vec::new();
        let mut phase = 0;
        loop {
            let mut bytes = [0; 32768];
            if let Ok(n) = master.read(&mut bytes) {
                output.extend_from_slice(&bytes[..n]);
            }
            let text = String::from_utf8_lossy(&output);
            if let Some((needle, keys)) = steps.get(phase)
                && text.contains(needle)
            {
                master.write_all(keys).unwrap();
                phase += 1;
                output.clear();
            }
            if let Some(status) = child.0.try_wait().unwrap() {
                assert_eq!(phase, steps.len());
                assert_eq!(status.code(), Some(expected_code));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "phase {phase}: {}",
                String::from_utf8_lossy(&output)
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut after = unsafe { std::mem::zeroed::<libc::termios>() };
        assert_eq!(unsafe { libc::tcgetattr(slave.as_raw_fd(), &mut after) }, 0);
        assert_eq!(
            before.c_lflag, after.c_lflag,
            "terminal restored before caller resumes"
        );
        let saved = if directory {
            dir.path().join("nested/note.md")
        } else {
            path
        };
        assert_eq!(std::fs::read_to_string(saved).unwrap(), expected);
        if comparison {
            assert_eq!(
                std::fs::read_to_string(dir.path().join("before.txt")).unwrap(),
                "Reference message\n"
            );
        }
    }
    #[test]
    fn comparison_switches_sides_then_saves_only_the_modified_file() {
        run_input(
            false,
            &[
                ("Original", b"["),
                ("ference", b"]"),
                ("Original", b"\x07"),
                ("EDIT", b"\x01Updated target\x13"),
                ("Saved", b"\x11"),
            ],
            0,
            "Updated target",
            None,
            &["--diff", "before.txt", "./-draft 日本語.txt"],
            false,
        );
    }
    #[test]
    fn piped_patch_projects_current_content_without_saving_or_applying_it() {
        run_input(
            false,
            &[("newcontent", b"\x11")],
            0,
            "Original message\n",
            Some(b"--- a\n+++ b\n@@ -1 +1 @@\n-oldcontent\n+newcontent\n"),
            &["-v", "--format", "diff"],
            false,
        );
    }
    #[test]
    fn caller_accepts_unchanged_document() {
        run(false, &[("EDIT", b"\x11")], 0, "Original message\n");
    }
    #[test]
    fn legacy_ctrl_e_moves_to_line_end_and_ctrl_g_switches_modes() {
        run(
            false,
            &[
                ("EDIT", b"\x05!!\x13"),
                ("Saved", b"\x07"),
                ("VIEW", b"g"),
                ("EDIT", b"\x11"),
            ],
            0,
            "Original message!!\n",
        );
    }
    #[test]
    fn caller_receives_abort_for_clean_document() {
        run(
            false,
            &[("EDIT", b"\x0bAbort editing\r"), ("Previous", b"a")],
            1,
            "Original message\n",
        );
    }
    #[test]
    fn caller_receives_abort_without_unsaved_changes_leaking() {
        run(
            false,
            &[("EDIT", b"CHANGED\x0bAbort editing\r"), ("Previous", b"a")],
            1,
            "Original message\n",
        );
    }
    #[test]
    fn directory_saves_relative_to_selected_directory() {
        run(
            true,
            &[
                ("Opening", b"\x01Hello directory\x13"),
                ("Save", b"nested/note.md\r"),
                ("Saved", b"\x11"),
            ],
            0,
            "Hello directory",
        );
    }

    #[test]
    fn delete_then_save_retained_text_through_real_terminal() {
        run(
            false,
            &[
                ("EDIT", b"\x0bDelete file permanently\r"),
                ("Permanently", b"d"),
                ("deleted;", b"\x13"),
                ("Save", "./-draft 日本語.txt\r".as_bytes()),
                ("Sav", b"\x11"),
            ],
            0,
            "Original message\n",
        );
    }

    #[test]
    fn caller_accepts_save_and_quit() {
        run(
            false,
            &[("EDIT", b"\x01Saved message\x13"), ("Saved", b"\x11")],
            0,
            "Saved message",
        );
    }

    #[test]
    fn abort_keeps_prior_save_and_cancel_does_not_exit() {
        run(
            false,
            &[
                ("EDIT", b"\x01Saved message\x13"),
                ("Saved", b"\x0bAbort editing\r"),
                ("Previous", b"\r"),
                ("?25h", b"unsaved\x0bAbort editing\r"),
                ("Previous", b"a"),
            ],
            1,
            "Saved message",
        );
    }

    #[test]
    fn ordinary_discard_is_success_not_abort() {
        run(
            false,
            &[("EDIT", b"unsaved\x11"), ("Unsaved changes", b"d")],
            0,
            "Original message\n",
        );
    }

    #[test]
    fn pipes_with_optional_dash_save_as_untitled_and_preserve_bytes() {
        for args in [vec![], vec!["-"], vec!["--format", "md", "-"]] {
            run_input(
                true,
                &[
                    ("EDIT", b"\x13"),
                    ("Save", b"nested/note.md\r"),
                    ("Saved", b"\x11"),
                ],
                0,
                "\u{feff}# Piped title\r\n",
                Some("\u{feff}# Piped title\r\n".as_bytes()),
                &args,
                false,
            );
        }
    }

    #[test]
    fn unchanged_pipe_quits_without_prompt_and_empty_pipe_is_valid() {
        for input in [b"".as_slice(), b"some piped text".as_slice()] {
            run_input(
                false,
                &[("EDIT", b"\x11")],
                0,
                "Original message\n",
                Some(input),
                &[],
                false,
            );
        }
    }

    #[test]
    fn unfinished_pipe_can_be_cancelled_from_terminal() {
        run_input(
            false,
            &[("Reading", b"\x1b")],
            1,
            "Original message\n",
            Some(b"partial"),
            &[],
            true,
        );
    }

    #[test]
    fn pipe_format_override_renders_markdown() {
        run_input(
            false,
            &[("VIEW", b"q")],
            0,
            "Original message\n",
            Some(b"# Piped title\n"),
            &["-v", "--format", "markdown"],
            false,
        );
    }

    #[test]
    fn invalid_or_ambiguous_pipe_fails_without_opening_a_partial_document() {
        for input in [b"\xff".as_slice(), b"a\0b".as_slice()] {
            run_input(false, &[], 1, "Original message\n", Some(input), &[], false);
        }
        run_input(
            false,
            &[],
            1,
            "Original message\n",
            Some(b"text"),
            &["file.txt"],
            false,
        );
    }
}
