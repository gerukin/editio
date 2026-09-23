use crossterm::event::{Event, KeyCode as K, KeyEvent, KeyModifiers as M};
use editio::{
    Action, Editor, Indentation, Mode,
    buffer::{Buffer, Cursor},
};
use ratatui::{Terminal, backend::TestBackend};
fn key(e: &mut Editor, c: K, m: M) {
    e.handle(Event::Key(KeyEvent::new(c, m)));
}
#[test]
fn brackets_indent_entire_selected_lines_and_undo() {
    let mut e = Editor::new(Buffer::new("one\ntwo\nthree"));
    e.mode = Mode::Edit;
    e.buffer.cursors = vec![Cursor { anchor: 1, head: 8 }];
    key(
        &mut e,
        K::Char(']'),
        if cfg!(target_os = "macos") {
            M::SUPER
        } else {
            M::CONTROL
        },
    );
    assert_eq!(e.buffer.text.to_string(), "\tone\n\ttwo\nthree");
    assert_eq!(e.buffer.cursors.len(), 1);
    assert_eq!(e.buffer.selected(), "ne\n\ttwo\n");
    key(
        &mut e,
        K::Char('['),
        if cfg!(target_os = "macos") {
            M::SUPER
        } else {
            M::CONTROL
        },
    );
    assert_eq!(e.buffer.text.to_string(), "one\ntwo\nthree");
    e.act(Action::Undo);
    assert_eq!(e.buffer.text.to_string(), "\tone\n\ttwo\nthree");
}
#[test]
fn tab_uses_stops_and_multiple_cursors_keep_their_heads() {
    let mut e = Editor::new(Buffer::new("a\nb"));
    e.mode = Mode::Edit;
    e.buffer.cursors = vec![Cursor::at(1), Cursor::at(2)];
    key(&mut e, K::Tab, M::NONE);
    assert_eq!(e.buffer.text.to_string(), "a\t\n\tb");
    assert_eq!(
        e.buffer.cursors.iter().map(|c| c.head).collect::<Vec<_>>(),
        [2, 4]
    );
}
#[test]
fn conversion_preserves_columns_interior_tabs_and_one_undo() {
    let mut e = Editor::new(Buffer::new("\t one\tvalue\n    two\nplain"));
    e.mode = Mode::Edit;
    e.act(Action::ConvertSpaces);
    assert_eq!(e.buffer.text.to_string(), "   one\tvalue\n    two\nplain");
    assert_eq!(e.buffer.cursors.len(), 1);
    e.act(Action::Undo);
    assert_eq!(e.buffer.text.to_string(), "\t one\tvalue\n    two\nplain");
    e.set_resolved_indentation(
        Indentation {
            tabs: true,
            size: 4,
            tab_width: 4,
        },
        "test".into(),
    );
    e.act(Action::ConvertDefault);
    // The explicit Spaces setting retains the old two-column tab width until conversion.
    assert_eq!(e.buffer.text.to_string(), "   one\tvalue\n\ttwo\nplain");
    assert_eq!(e.buffer.tab_width, 4);
}
#[test]
fn configured_tabs_align_render_cursor_and_mouse() {
    let mut e = Editor::new(Buffer::new("\tx\n\ty"));
    e.mode = Mode::Edit;
    e.wrap = false;
    e.buffer.cursors = vec![Cursor::at(1)];
    let mut t = Terminal::new(TestBackend::new(30, 8)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    assert_eq!(e.buffer.display_column(1), 2);
    assert_eq!(e.buffer.position_at_column(0, 2), 1);
    let b = t.backend().buffer();
    assert_eq!(b[(2, 0)].symbol(), "x");
    e.override_indentation(Indentation {
        tabs: false,
        size: 3,
        tab_width: 6,
    });
    t.draw(|f| e.draw(f, f.area())).unwrap();
    assert_eq!(t.backend().buffer()[(6, 0)].symbol(), "x");
    assert!(!e.buffer.dirty());
}
#[cfg(target_os = "macos")]
#[test]
fn native_preferences_replacement_on_disposable_copy() {
    let Ok(path) = std::env::var("EDITIO_PREFS_TEST_COPY") else {
        return;
    };
    let path = std::path::Path::new(&path);
    let before = std::fs::read(path).unwrap();
    for _ in 0..3 {
        tapp_ui::storage::update(path, |old| Ok(old.unwrap().to_vec())).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
    for _ in 0..4 {
        let mut editor = Editor::new(Buffer::new("text"));
        editor.buffer.path = Some(path.with_extension("md"));
        editor.enable_preferences(path).unwrap();
        let previous = editor.limit_width;
        editor.act(Action::ToggleWidthLimit);
        assert_ne!(editor.limit_width, previous);
        let mut reopened = Editor::new(Buffer::new("text"));
        reopened.buffer.path = editor.buffer.path.clone();
        reopened.enable_preferences(path).unwrap();
        assert_eq!(
            reopened.limit_width, editor.limit_width,
            "{}",
            editor.message
        );
    }
}

#[test]
fn remembered_defaults_do_not_persist_document_overrides_accidentally() {
    let dir = tempfile::tempdir().unwrap();
    let prefs = dir.path().join("config.json");
    let mut e = Editor::new(Buffer::new("text"));
    e.buffer.path = Some(dir.path().join("one.md"));
    e.enable_preferences(&prefs).unwrap();
    e.override_indentation(Indentation {
        tabs: false,
        size: 3,
        tab_width: 3,
    });
    e.act(Action::IndentSaveDefault);
    let mut other = Editor::new(Buffer::new("text"));
    other.buffer.path = Some(dir.path().join("two.md"));
    other.enable_preferences(&prefs).unwrap();
    assert_eq!(other.indentation().size, 3);
    assert!(!other.indentation().tabs);
    other.override_indentation(Indentation {
        tabs: true,
        size: 8,
        tab_width: 8,
    });
    other.act(Action::ToggleWrap);
    let mut again = Editor::new(Buffer::new("text"));
    again.buffer.path = Some(dir.path().join("three.md"));
    again.enable_preferences(&prefs).unwrap();
    assert_eq!(again.indentation().size, 3);
}

#[test]
#[ignore = "manual release performance measurement"]
fn measure_large_indentation_conversion() {
    let source = "    code with an interior\ttab\n".repeat(100_000);
    let mut buffer = Buffer::new(&source);
    let now = std::time::Instant::now();
    buffer.indent_lines(Indentation::default(), false, true);
    eprintln!(
        "100,000 lines / {} bytes converted in {:?}",
        source.len(),
        now.elapsed()
    );
    assert_eq!(buffer.cursors.len(), 1);
    assert!(buffer.text.to_string().starts_with("\t\tcode"));
    buffer.undo();
    assert_eq!(buffer.text.to_string(), source);
    #[cfg(target_os = "linux")]
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(line) = status.lines().find(|line| line.starts_with("VmHWM:"))
    {
        eprintln!("{line}");
    }
}

#[test]
fn conversion_commands_are_discoverable_in_edit_on_every_platform() {
    let mut e = Editor::new(Buffer::new("\ttext"));
    for mode in [Mode::View, Mode::Edit] {
        e.mode = mode;
        e.act(Action::Commands);
        e.prompt.as_mut().unwrap().value = "convert indentation".into();
        let actions: Vec<_> = e.matching_commands().iter().map(|c| c.action).collect();
        if mode == Mode::Edit {
            assert_eq!(
                actions,
                [
                    Action::ConvertSpaces,
                    Action::ConvertTabs,
                    Action::ConvertDefault
                ]
            );
        } else {
            assert!(actions.is_empty());
        }
        e.prompt = None;
    }
}
