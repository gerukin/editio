use crossterm::event::{
    Event, KeyCode as K, KeyEvent, KeyModifiers as M, MouseEvent, MouseEventKind,
};
use editio::{
    Action, Editor, Mode, Outcome,
    buffer::{Buffer, Cursor},
};
use ratatui::{Terminal, backend::TestBackend};

fn draw(editor: &mut Editor) {
    let mut terminal = Terminal::new(TestBackend::new(24, 10)).unwrap();
    terminal.draw(|f| editor.draw(f, f.area())).unwrap();
}

#[test]
fn wheel_gesture_cannot_drift_diagonally_but_keys_are_independent() {
    let mut editor = Editor::new(Buffer::new(
        &"a long line that needs horizontal scrolling\n".repeat(40),
    ));
    editor.mode = Mode::View;
    editor.wrap = false;
    draw(&mut editor);
    let wheel = |kind| {
        Event::Mouse(MouseEvent {
            kind,
            column: 5,
            row: 5,
            modifiers: M::NONE,
        })
    };
    editor.handle(wheel(MouseEventKind::ScrollDown));
    assert_eq!(editor.scroll, 3);
    editor.handle(wheel(MouseEventKind::ScrollRight));
    assert_eq!(editor.horizontal, 0);
    editor.handle(Event::Key(KeyEvent::new(K::Right, M::NONE)));
    assert_eq!(editor.horizontal, 1);
    editor.handle(wheel(MouseEventKind::ScrollRight));
    assert_eq!(editor.horizontal, 5);
    editor.handle(wheel(MouseEventKind::ScrollDown));
    assert_eq!(editor.scroll, 3);
}

#[test]
fn clipboard_paste_in_find_does_not_edit_underlying_document() {
    let mut editor = Editor::new(Buffer::new("original document"));
    editor.mode = Mode::Edit;
    editor.act(Action::Find);
    assert_eq!(
        editor.handle(Event::Key(KeyEvent::new(K::Char('v'), M::CONTROL))),
        Outcome::PasteRequested
    );
    editor.paste("needle");
    assert_eq!(editor.prompt.as_ref().unwrap().value, "needle");
    assert_eq!(editor.buffer.text.to_string(), "original document");
}

#[test]
fn physical_home_end_work_for_all_wrapped_cursors() {
    let line = "long line with enough words to wrap across several viewport rows";
    let mut editor = Editor::new(Buffer::new(&format!("{line}\n{line}")));
    editor.mode = Mode::Edit;
    editor.wrap = true;
    editor.buffer.cursors = vec![Cursor::at(35), Cursor::at(line.len() + 36)];
    draw(&mut editor);
    editor.handle(Event::Key(KeyEvent::new(K::Home, M::NONE)));
    assert_eq!(
        editor
            .buffer
            .cursors
            .iter()
            .map(|c| c.head)
            .collect::<Vec<_>>(),
        [0, line.len() + 1]
    );
    editor.handle(Event::Key(KeyEvent::new(K::End, M::SHIFT)));
    assert_eq!(editor.buffer.cursors[0].range(), 0..line.len());
    assert_eq!(
        editor.buffer.cursors[1].range(),
        line.len() + 1..line.len() * 2 + 1
    );
}

#[test]
fn mode_switch_uses_only_ctrl_g_and_plain_g_outside_inputs() {
    let mut editor = Editor::new(Buffer::new("one line\nsecond"));
    editor.mode = Mode::View;
    for key in [K::Char('e'), K::Char('m'), K::Enter] {
        editor.handle(Event::Key(KeyEvent::new(key, M::NONE)));
        assert_eq!(editor.mode, Mode::View);
    }
    editor.handle(Event::Key(KeyEvent::new(K::Char('g'), M::NONE)));
    assert_eq!(editor.mode, Mode::Edit);
    editor.handle(Event::Key(KeyEvent::new(K::Char('g'), M::NONE)));
    assert_eq!(editor.buffer.text.to_string(), "gone line\nsecond");
    editor.handle(Event::Key(KeyEvent::new(K::Char('e'), M::CONTROL)));
    assert_eq!(editor.mode, Mode::Edit);
    assert_eq!(editor.buffer.cursors[0].head, 9);
    editor.handle(Event::Key(KeyEvent::new(K::Char('g'), M::CONTROL)));
    assert_eq!(editor.mode, Mode::View);
    editor.act(Action::Find);
    editor.handle(Event::Key(KeyEvent::new(K::Char('g'), M::NONE)));
    assert_eq!(editor.prompt.as_ref().unwrap().value, "g");
    editor.handle(Event::Key(KeyEvent::new(K::Char('g'), M::CONTROL)));
    assert_eq!(editor.mode, Mode::View);
    assert!(editor.prompt.is_some());
}

#[test]
fn ctrl_e_extends_each_cursor_to_its_physical_line_end() {
    let mut editor = Editor::new(Buffer::new(
        "first line that wraps many times\nsecond long line here",
    ));
    editor.mode = Mode::Edit;
    editor.wrap = true;
    let second = editor.buffer.text.line_to_char(1);
    editor.buffer.cursors = vec![Cursor::at(2), Cursor::at(second + 3)];
    draw(&mut editor);
    editor.handle(Event::Key(KeyEvent::new(
        K::Char('e'),
        M::CONTROL | M::SHIFT,
    )));
    assert_eq!(editor.buffer.cursors[0].range(), 2..second - 1);
    assert_eq!(
        editor.buffer.cursors[1].range(),
        second + 3..editor.buffer.text.len_chars()
    );
    assert_eq!(editor.mode, Mode::Edit);
}

#[test]
fn ctrl_a_has_platform_specific_meaning_without_changing_cmd_a() {
    let mut editor = Editor::new(Buffer::new("first\nsecond line"));
    editor.mode = Mode::Edit;
    editor.buffer.cursors = vec![Cursor::at(10)];
    editor.handle(Event::Key(KeyEvent::new(K::Char('a'), M::CONTROL)));
    #[cfg(target_os = "macos")]
    {
        assert_eq!(editor.buffer.cursors[0].head, 6);
        assert!(editor.buffer.cursors[0].range().is_empty());
        editor.handle(Event::Key(KeyEvent::new(K::Char('a'), M::SUPER)));
    }
    assert_eq!(editor.buffer.cursors[0].range(), 0..17);
}

#[cfg(target_os = "macos")]
#[test]
fn mac_command_keys_copy_paste_navigate_and_respect_handoff() {
    let line = "a long line with words that wrap across several screen rows";
    let mut editor = Editor::new(Buffer::new(line));
    editor.mode = Mode::Edit;
    editor.wrap = true;
    editor.buffer.cursors = vec![Cursor::at(32)];
    draw(&mut editor);
    editor.handle(Event::Key(KeyEvent::new(K::Left, M::SUPER)));
    assert_eq!(editor.buffer.cursors[0].head, 0);
    editor.handle(Event::Key(KeyEvent::new(K::Right, M::SUPER | M::SHIFT)));
    assert_eq!(editor.buffer.cursors[0].range(), 0..line.len());
    assert_eq!(
        editor.handle(Event::Key(KeyEvent::new(K::Char('c'), M::SUPER))),
        Outcome::CopyRequested(line.into())
    );
    assert_eq!(
        editor.handle(Event::Key(KeyEvent::new(K::Char('v'), M::SUPER))),
        Outcome::PasteRequested
    );
    let key = KeyEvent::new(K::Left, M::SUPER);
    editor.handoff_keys.push(key);
    assert_eq!(editor.handle(Event::Key(key)), Outcome::FocusReleased(key));
}
