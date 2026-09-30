use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers as M, MouseButton as B, MouseEvent, MouseEventKind as MK,
};
use editio::{
    Action, Editor, Mode, Outcome,
    buffer::{Buffer, Cursor},
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn mouse(e: &mut Editor, kind: MK, column: u16, row: u16, modifiers: M) -> Outcome {
    e.handle(Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    }))
}

#[test]
fn shift_click_keeps_anchor_across_unicode_tabs_reverse_selection_and_release() {
    for mode in [Mode::Edit, Mode::View] {
        for wrap in [false, true] {
            let mut e = Editor::new(Buffer::new("猫\tabc\nsecond"));
            e.mode = mode;
            e.wrap = wrap;
            e.buffer.tab_width = 4;
            e.buffer.cursors = vec![Cursor { anchor: 3, head: 5 }, Cursor::at(8)];
            e.set_copy_on_selection(true);
            let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
            terminal
                .draw(|f| e.draw(f, Rect::new(4, 3, 20, 8)))
                .unwrap();
            // Hit the second display cell of 猫, then a position on the next line.
            for (column, row, head, selected) in [(5, 3, 0, "猫\ta"), (7, 4, 9, "bc\nsec")] {
                mouse(&mut e, MK::Down(B::Left), column, row, M::SHIFT);
                assert_eq!(e.buffer.cursors, vec![Cursor { anchor: 3, head }]);
                assert!(e.poll_selection_copy().is_none());
                mouse(&mut e, MK::Up(B::Left), column, row, M::SHIFT);
                assert_eq!(e.poll_selection_copy().as_deref(), Some(selected));
            }
            // A subsequent drag continues to use the same anchor.
            mouse(&mut e, MK::Down(B::Left), 8, 4, M::SHIFT);
            mouse(&mut e, MK::Drag(B::Left), 4, 4, M::SHIFT);
            assert_eq!(e.buffer.cursors, vec![Cursor { anchor: 3, head: 6 }]);
            assert_eq!(e.act(Action::Copy), Outcome::CopyRequested("bc\n".into()));
        }
    }
}

#[test]
fn ctrl_option_arrows_add_cursors_and_preserve_document_text() {
    let source = "abc\ndef\nghi";
    for (code, position) in [(KeyCode::Up, 9), (KeyCode::Down, 1)] {
        let mut e = Editor::new(Buffer::new(source));
        e.mode = Mode::Edit;
        e.buffer.cursors = vec![Cursor::at(position)];
        for _ in 0..3 {
            e.handle(Event::Key(KeyEvent::new(code, M::CONTROL | M::ALT)));
        }
        assert_eq!(e.buffer.cursors.len(), 3);
        assert_eq!(e.buffer.text.to_string(), source);
        e.handle(Event::Key(KeyEvent::new(KeyCode::Char('X'), M::NONE)));
        assert_eq!(e.buffer.text.to_string(), "aXbc\ndXef\ngXhi");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn forwarded_command_option_arrows_add_cursors_without_becoming_navigation() {
    for (code, position) in [(KeyCode::Up, 9), (KeyCode::Down, 1)] {
        let mut e = Editor::new(Buffer::new("abc\ndef\nghi"));
        e.mode = Mode::Edit;
        e.buffer.cursors = vec![Cursor::at(position)];
        for _ in 0..2 {
            e.handle(Event::Key(KeyEvent::new(code, M::SUPER | M::ALT)));
        }
        assert_eq!(e.buffer.cursors.len(), 3);
        assert_eq!(e.buffer.text.to_string(), "abc\ndef\nghi");
    }
}
