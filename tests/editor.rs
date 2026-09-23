use crossterm::event::{
    Event, KeyCode as K, KeyEvent, KeyModifiers as M, MouseButton, MouseEvent, MouseEventKind,
};
use editio::{
    Action, Editor, Mode, Outcome,
    buffer::{Buffer, Cursor},
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
fn key(e: &mut Editor, k: K, m: M) -> Outcome {
    e.handle(Event::Key(KeyEvent::new(k, m)))
}
fn draw(e: &mut Editor, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    let b = t.backend().buffer();
    (0..h)
        .map(|y| (0..w).map(|x| b[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn multi_cursor_unicode_edit_and_undo() {
    let mut b = Buffer::new("猫 cat\n猫 cat");
    b.cursors = vec![Cursor { anchor: 0, head: 1 }, Cursor { anchor: 6, head: 7 }];
    b.insert("犬🐕");
    assert_eq!(b.text.to_string(), "犬🐕 cat\n犬🐕 cat");
    b.undo();
    assert_eq!(b.text.to_string(), "猫 cat\n猫 cat");
    assert!(!b.dirty());
    b.redo();
    assert_eq!(b.cursors.len(), 2);
}
#[test]
fn grapheme_deletion() {
    let mut b = Buffer::new("a👩‍💻e\u{301}");
    b.cursors = vec![Cursor::at(b.text.len_chars())];
    b.delete(false);
    assert_eq!(b.text.to_string(), "a👩‍💻");
    b.delete(false);
    assert_eq!(b.text.to_string(), "a");
}
#[test]
fn overlapping_selections_are_merged() {
    let mut b = Buffer::new("abcdef");
    b.cursors = vec![Cursor { anchor: 0, head: 3 }, Cursor { anchor: 2, head: 5 }];
    b.insert("X");
    assert_eq!(b.text.to_string(), "Xf");
}
#[test]
fn safe_save_preserves_crlf_bom_and_detects_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::write(&p, "\u{feff}one\r\ntwo\r\n").unwrap();
    let mut b = Buffer::open(&p).unwrap();
    b.insert("猫");
    b.save().unwrap();
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        "\u{feff}猫one\r\ntwo\r\n"
    );
    std::fs::write(&p, "external").unwrap();
    b.insert("x");
    assert!(
        b.save()
            .unwrap_err()
            .to_string()
            .contains("changed on disk")
    );
    assert_eq!(std::fs::read_to_string(p).unwrap(), "external");
}
#[test]
fn view_mode_is_read_only() {
    let mut e = Editor::new(Buffer::new("hello"));
    key(&mut e, K::Char('x'), M::NONE);
    key(&mut e, K::Backspace, M::NONE);
    e.handle(Event::Paste("bad".into()));
    assert_eq!(e.buffer.text.to_string(), "hello");
}
#[test]
fn host_handoff_precedes_modal_and_edit() {
    let mut e = Editor::new(Buffer::new("hello"));
    e.mode = Mode::Edit;
    e.handoff_keys.push(KeyEvent::new(K::Enter, M::NONE));
    key(&mut e, K::F(2), M::NONE);
    assert!(matches!(
        key(&mut e, K::Enter, M::NONE),
        Outcome::FocusReleased(_)
    ));
    assert_eq!(e.buffer.text.to_string(), "hello");
    assert!(e.prompt.is_none());
    assert!(matches!(
        key(&mut e, K::Esc, M::NONE),
        Outcome::FocusReleased(_)
    ));
}

#[test]
fn ctrl_g_toggles_modes_preserving_edits_selection_and_undo() {
    let mut e = Editor::new(Buffer::new("hello"));
    key(&mut e, K::Char('g'), M::CONTROL);
    assert_eq!(e.mode, Mode::Edit);
    key(&mut e, K::Char('!'), M::NONE);
    e.buffer.cursors = vec![Cursor { anchor: 1, head: 4 }];
    key(&mut e, K::Char('g'), M::CONTROL);
    assert_eq!(e.mode, Mode::View);
    assert_eq!(e.buffer.text.to_string(), "!hello");
    assert_eq!(e.buffer.cursors[0].range(), 1..4);
    key(&mut e, K::Char('g'), M::CONTROL);
    assert_eq!(e.mode, Mode::Edit);
    key(&mut e, K::Char('z'), M::CONTROL);
    assert_eq!(e.buffer.text.to_string(), "hello");
}

#[test]
fn ctrl_g_respects_modal_focus_release_events_and_host_handoff() {
    let mut e = Editor::new(Buffer::new("hello"));
    key(&mut e, K::Char('f'), M::CONTROL);
    key(&mut e, K::Char('g'), M::CONTROL);
    assert!(e.prompt.is_some());
    assert_eq!(e.mode, Mode::View);
    key(&mut e, K::Esc, M::NONE);
    e.handle(Event::Key(KeyEvent::new_with_kind(
        K::Char('g'),
        M::CONTROL,
        crossterm::event::KeyEventKind::Release,
    )));
    assert_eq!(e.mode, Mode::View);
    key(&mut e, K::Char('g'), M::CONTROL | M::ALT);
    assert_eq!(e.mode, Mode::View);
    e.handoff_keys.push(KeyEvent::new(K::Char('g'), M::CONTROL));
    assert!(matches!(
        key(&mut e, K::Char('g'), M::CONTROL),
        Outcome::FocusReleased(_)
    ));
    assert_eq!(e.mode, Mode::View);
}
#[test]
fn palette_filters_and_executes() {
    let mut e = Editor::new(Buffer::new("hello"));
    key(&mut e, K::Char('k'), M::CONTROL);
    for c in "toggle view".chars() {
        key(&mut e, K::Char(c), M::NONE);
    }
    assert_eq!(e.matching_commands().len(), 1);
    key(&mut e, K::Enter, M::NONE);
    assert_eq!(e.mode, Mode::Edit);
    assert_eq!(e.buffer.text.to_string(), "hello");
}
#[test]
fn mouse_offset_wide_char_selection_and_modal_isolation() {
    let mut e = Editor::new(Buffer::new("猫abc"));
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    t.draw(|f| e.draw(f, Rect::new(10, 5, 50, 12))).unwrap();
    let click = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 12,
        row: 6,
        modifiers: M::NONE,
    });
    e.handle(click.clone());
    assert_eq!(e.buffer.cursors[0].head, 1);
    key(&mut e, K::F(2), M::NONE);
    e.handle(Event::Mouse(MouseEvent {
        column: 14,
        ..match click {
            Event::Mouse(m) => m,
            _ => unreachable!(),
        }
    }));
    assert_eq!(e.buffer.cursors[0].head, 1);
}
#[test]
fn next_occurrences_and_edit() {
    let mut e = Editor::new(Buffer::new("cat dog cat"));
    e.mode = Mode::Edit;
    e.act(Action::AddNext);
    e.act(Action::AddNext);
    key(&mut e, K::Char('X'), M::NONE);
    assert_eq!(e.buffer.text.to_string(), "X dog X");
}
#[test]
fn resize_and_palette_empty_state() {
    let mut e = Editor::new(Buffer::new("hello 猫\nworld"));
    for (w, h) in [(1, 1), (15, 3), (16, 4), (30, 8), (80, 24), (240, 80)] {
        let s = draw(&mut e, w, h);
        if w >= 30 {
            assert!(s.contains("VIEW"));
        }
    }
    key(&mut e, K::F(2), M::NONE);
    for c in "zzzzzzzzzz".chars() {
        key(&mut e, K::Char(c), M::NONE);
    }
    assert!(draw(&mut e, 80, 24).contains("No matching commands"));
}
#[test]
fn prompt_unicode_editing_and_paste() {
    let mut e = Editor::new(Buffer::new("猫"));
    e.act(Action::Find);
    e.handle(Event::Paste("犬猫".into()));
    key(&mut e, K::Home, M::NONE);
    key(&mut e, K::Delete, M::NONE);
    assert_eq!(e.prompt.as_ref().unwrap().value, "猫");
    key(&mut e, K::Enter, M::NONE);
    assert_eq!(e.buffer.selected(), "猫");
}
#[test]
fn new_file_and_invalid_encoding() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("new");
    let mut b = Buffer::open(&p).unwrap();
    b.insert("new");
    b.save().unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
    std::fs::write(&p, [0xff]).unwrap();
    assert!(Buffer::open(&p).is_err());
}

#[test]
fn markdown_and_mermaid_render_in_background() {
    let mut e =
        Editor::new(Buffer::open(std::path::Path::new("tests/fixtures/showcase.md")).unwrap());
    draw(&mut e, 100, 100);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let text = draw(&mut e, 100, 100);
    assert!(text.contains("Open file"), "{text}");
    assert!(text.contains("Alice"));
    assert!(text.contains("Hello"), "{text}");
    assert!(text.contains("▼") || text.contains("↓") || text.contains("v"));
    assert!(!text.contains("A[Open file] -->"));
    std::fs::write("target/preview.txt", text).unwrap();
}
#[test]
fn adjacent_selections_remain_distinct_and_empty_edits_are_clean() {
    let mut b = Buffer::new("ab");
    b.delete(false);
    assert!(!b.dirty());
    b.cursors = vec![Cursor { anchor: 0, head: 1 }, Cursor { anchor: 1, head: 2 }];
    b.insert("x");
    assert_eq!(b.text.to_string(), "xx");
}
#[test]
fn replace_is_one_undoable_transaction() {
    let mut e = Editor::new(Buffer::new("猫 cat 猫"));
    e.mode = Mode::Edit;
    e.act(Action::Replace);
    e.handle(Event::Paste("猫".into()));
    key(&mut e, K::Enter, M::NONE);
    e.handle(Event::Paste("dog".into()));
    key(&mut e, K::Enter, M::NONE);
    assert_eq!(e.buffer.text.to_string(), "dog cat dog");
    e.act(Action::Undo);
    assert_eq!(e.buffer.text.to_string(), "猫 cat 猫");
}
#[test]
fn line_actions_and_bracket_navigation() {
    let mut e = Editor::new(Buffer::new("a\nb\n"));
    e.mode = Mode::Edit;
    e.act(Action::MoveDown);
    assert_eq!(e.buffer.text.to_string(), "b\na\n");
    e.act(Action::Undo);
    e.act(Action::Indent);
    assert_eq!(e.buffer.text.to_string(), "\ta\nb\n");
    e.act(Action::Outdent);
    assert_eq!(e.buffer.text.to_string(), "a\nb\n");
    let mut e = Editor::new(Buffer::new("{[()]}"));
    e.act(Action::MatchBracket);
    assert_eq!(e.buffer.cursors[0].head, 5);
    e.act(Action::MatchBracket);
    assert_eq!(e.buffer.cursors[0].head, 0);
}
#[test]
fn palette_scroll_and_mouse_activation() {
    let mut e = Editor::new(Buffer::new("a"));
    key(&mut e, K::Char('k'), M::CONTROL);
    key(&mut e, K::Tab, M::NONE);
    key(&mut e, K::PageDown, M::NONE);
    assert!(draw(&mut e, 40, 10).contains("Command center"));
    let mut e = Editor::new(Buffer::new("a"));
    key(&mut e, K::Char('k'), M::CONTROL);
    draw(&mut e, 80, 24);
    // Palette and options clicks cannot edit the underlying buffer.
    e.handle(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 3,
        modifiers: M::NONE,
    }));
    assert_eq!(e.buffer.text.to_string(), "a");
}
#[test]
fn mixed_line_endings_are_never_silently_rewritten() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("mixed");
    std::fs::write(&p, "a\r\nb\n").unwrap();
    assert!(Buffer::open(&p).is_err());
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "a\r\nb\n");
}

#[test]
fn select_word_from_middle_then_next_and_collapse() {
    let mut e = Editor::new(Buffer::new("hello hello"));
    e.mode = Mode::Edit;
    e.buffer.cursors[0] = Cursor::at(2);
    e.act(Action::AddNext);
    assert_eq!(e.buffer.selected(), "hello");
    assert_eq!(e.buffer.cursors.len(), 1);
    e.act(Action::AddNext);
    assert_eq!(e.buffer.cursors.len(), 2);
    key(&mut e, K::Left, M::NONE);
    assert_eq!(e.buffer.cursors[0].head, 0);
    assert_eq!(e.buffer.cursors[1].head, 6);
}
