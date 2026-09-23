use editio::{Action, Editor, Mode, buffer::Buffer};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use tapp_ui::theme::Theme;

#[test]
fn shared_theme_is_opt_in_persistent_and_reversible_without_reformatting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("theme");
    let mut editor = Editor::new(Buffer::new("Hello\nworld"));
    editor.mode = Mode::View;
    assert_eq!(editor.theme, Theme::Terminal);
    editor.enable_theme_preferences(&path).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(70, 12)).unwrap();
    terminal.draw(|f| editor.draw(f, f.area())).unwrap();
    let before = terminal.backend().buffer().clone();
    editor.act(Action::ToggleTheme);
    terminal.draw(|f| editor.draw(f, f.area())).unwrap();
    let fixed = terminal.backend().buffer().clone();
    assert_eq!(fixed[(0, 0)].bg, Color::Rgb(0x1a, 0x1b, 0x26));
    assert_eq!(fixed[(0, 11)].fg, Color::Rgb(0x1a, 0x1b, 0x26));
    assert_eq!(fixed[(0, 11)].bg, Color::Rgb(107, 141, 215));
    assert!(
        !fixed[(0, 11)]
            .modifier
            .intersects(ratatui::style::Modifier::DIM | ratatui::style::Modifier::REVERSED)
    );
    assert_eq!(
        before
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>(),
        fixed.content.iter().map(|c| c.symbol()).collect::<String>()
    );
    let mut other = Editor::new(Buffer::new("different file"));
    other.enable_theme_preferences(&path).unwrap();
    assert_eq!(other.theme, Theme::TokyoNightOmarchy);
    editor.act(Action::ToggleTheme);
    terminal.draw(|f| editor.draw(f, f.area())).unwrap();
    assert_eq!(*terminal.backend().buffer(), before);
    assert!(!editor.buffer.dirty());
}

#[cfg(unix)]
#[test]
fn editor_preferences_keep_metadata_across_repeated_toggles() {
    use std::os::fd::AsRawFd;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut editor = Editor::new(Buffer::new(""));
    editor.enable_preferences(&path).unwrap();
    editor.act(Action::ToggleTheme);
    let file = std::fs::File::open(&path).unwrap();
    let name = c"user.editio_test";
    #[cfg(target_os = "linux")]
    let rc = unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            name.as_ptr(),
            b"keep".as_ptr().cast(),
            4,
            0,
        )
    };
    #[cfg(target_os = "macos")]
    let rc = unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            name.as_ptr(),
            b"keep".as_ptr().cast(),
            4,
            0,
            0,
        )
    };
    assert_eq!(rc, 0);
    for _ in 0..3 {
        editor.act(Action::ToggleTheme);
        let mut check = Editor::new(Buffer::new(""));
        check.enable_preferences(&path).unwrap();
        assert_eq!(check.theme, editor.theme, "{}", editor.message);
        let file = std::fs::File::open(&path).unwrap();
        let mut value = [0; 4];
        #[cfg(target_os = "linux")]
        let rc = unsafe {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
            )
        };
        #[cfg(target_os = "macos")]
        let rc = unsafe {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
                0,
                0,
            )
        };
        assert_eq!(rc, 4);
        assert_eq!(&value, b"keep");
    }
}
