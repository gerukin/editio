use crossterm::event::{
    Event, KeyCode as K, KeyEvent, KeyModifiers as M, MouseButton, MouseEvent, MouseEventKind as MK,
};
use editio::{Action, Editor, Mode, Outcome, buffer::Buffer};
use ratatui::{
    Terminal, backend::TestBackend, buffer::Buffer as Screen, layout::Rect, style::Modifier,
};

fn editor(source: &str, file: &str, mode: Mode) -> Editor {
    let mut buffer = Buffer::new(source);
    buffer.path = Some(file.into());
    let mut e = Editor::new(buffer);
    e.mode = mode;
    e
}
fn draw(e: &mut Editor, width: u16) -> Screen {
    let mut terminal = Terminal::new(TestBackend::new(90, 42)).unwrap();
    terminal
        .draw(|f| e.draw(f, Rect::new(3, 3, width, 35)))
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(2));
        terminal
            .draw(|f| e.draw(f, Rect::new(3, 3, width, 35)))
            .unwrap();
    }
    terminal.backend().buffer().clone()
}
fn text(screen: &Screen, row: u16, width: u16) -> String {
    (3..3 + width).map(|x| screen[(x, row)].symbol()).collect()
}
fn copy(e: &mut Editor) -> String {
    match e.act(Action::Copy) {
        Outcome::CopyRequested(s) => s,
        other => panic!("{other:?}"),
    }
}
fn mouse(e: &mut Editor, kind: MK, x: u16, y: u16) {
    e.handle(Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: M::NONE,
    }));
}

#[test]
fn wrapped_source_mouse_selection_ignores_virtual_indentation() {
    let source = "  - [x] alpha beta gamma delta epsilon zeta eta theta";
    for mode in [Mode::Edit, Mode::View] {
        let mut e = editor(source, "words.txt", mode);
        let screen = draw(&mut e, 25);
        assert!(text(&screen, 4, 25).starts_with("        "));
        let continuation = text(&screen, 4, 25).trim().to_owned();
        mouse(&mut e, MK::Down(MouseButton::Left), 3, 4);
        assert_eq!(
            e.buffer.cursors[0].head,
            source.find(&continuation).unwrap()
        );
        assert_eq!(e.mode, mode);
        mouse(&mut e, MK::Down(MouseButton::Left), 3, 3);
        let last = (3..37)
            .rev()
            .find(|&row| !text(&screen, row, 25).trim().is_empty())
            .unwrap();
        mouse(&mut e, MK::Drag(MouseButton::Left), 27, last);
        assert_eq!(copy(&mut e), source);
        let selected = draw(&mut e, 25);
        assert!(!selected[(3, 4)].modifier.contains(Modifier::REVERSED));
        e.act(Action::ToggleWrap);
        draw(&mut e, 25);
        assert_eq!(copy(&mut e), source);
    }
}

#[test]
fn long_tokens_fill_remaining_space_identically_in_preview_and_edit() {
    fn visible(screen: &Screen, row: u16, width: u16) -> String {
        use unicode_width::UnicodeWidthStr;
        let mut x = 3;
        let mut result = String::new();
        while x < 3 + width {
            let symbol = screen[(x, row)].symbol();
            result.push_str(symbol);
            x += symbol.width().max(1) as u16;
        }
        result.trim_end().to_owned()
    }
    for source in [
        "Prefix abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 tail",
        "Prefix 日本語日本語日本語日本語日本語日本語日本語日本語日本語 tail",
        "- Prefix abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 tail",
    ] {
        for width in [20, 31, 45] {
            let mut view = editor(source, "tokens.md", Mode::View);
            let mut edit = editor(source, "tokens.md", Mode::Edit);
            let rendered = draw(&mut view, width);
            let raw = draw(&mut edit, width);
            for row in 3..15 {
                let expected = visible(&raw, row, width);
                let expected = if row == 3 && source.starts_with("- ") {
                    expected.replacen("- ", "○ ", 1)
                } else {
                    expected
                };
                assert_eq!(
                    visible(&rendered, row, width),
                    expected,
                    "{source} width {width} row {row}"
                );
            }
            assert!(text(&raw, 3, width).trim_end().len() > "- Prefix ".len());
            for e in [&mut view, &mut edit] {
                e.act(Action::SelectAll);
                let expected = if e.mode == Mode::View && source.starts_with("- ") {
                    source.replacen("- ", "○ ", 1)
                } else {
                    source.to_owned()
                };
                assert_eq!(copy(e), expected);
            }
        }
    }
}

#[test]
fn preview_wrap_toggle_keeps_copy_and_list_indentation_and_protects_geometry() {
    let source = "- [x] A long task with **bold text** and [a link](https://example.com) that wraps onto several rows.\n\n> A quoted paragraph that also wraps onto multiple rows while keeping a border.\n\n```text\ncode line that is deliberately much wider than the viewport\n```\n";
    let mut e = editor(source, "wrap.md", Mode::View);
    let wrapped = draw(&mut e, 28);
    let continuation = text(&wrapped, 4, 28);
    assert_eq!(continuation.len() - continuation.trim_start().len(), 2);
    e.act(Action::SelectAll);
    let selected = copy(&mut e);
    assert!(!selected.contains('│'));
    assert!(
        selected.contains("bold text and a link that wraps"),
        "{selected:?}"
    );
    e.act(Action::ToggleWrap);
    assert!(!e.wrap);
    draw(&mut e, 28);
    e.act(Action::SelectAll);
    assert_eq!(copy(&mut e), selected);
    e.act(Action::ToggleMode);
    assert!(!e.wrap);
    draw(&mut e, 28);
    e.act(Action::ToggleWrap);
    draw(&mut e, 28);
    assert_eq!(e.buffer.text.to_string(), source);
}

#[test]
fn visual_navigation_resize_and_toggle_shortcut_preserve_source() {
    let source = "    alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let mut e = editor(source, "notes.txt", Mode::Edit);
    draw(&mut e, 24);
    e.handle(Event::Key(KeyEvent::new(K::Down, M::SHIFT)));
    assert!(e.buffer.cursors[0].head > 0);
    let selected = copy(&mut e);
    assert_eq!(selected, &source[..e.buffer.cursors[0].head]);
    let head = e.buffer.cursors[0].head;
    draw(&mut e, 40);
    assert_eq!(e.buffer.cursors[0].head, head);
    e.handle(Event::Key(KeyEvent::new(K::Char('k'), M::CONTROL)));
    e.handle(Event::Paste("word wrap".into()));
    e.handle(Event::Key(KeyEvent::new(K::Enter, M::NONE)));
    assert!(!e.wrap);
    draw(&mut e, 24);
    assert_eq!(copy(&mut e), selected);
    let mut csv = editor("a,b\nc,d\n", "data.csv", Mode::Edit);
    assert!(!csv.wrapping_supported());
    csv.act(Action::ToggleWrap);
    assert!(csv.message.contains("prose"));
}

#[test]
fn wrapped_end_reaches_source_line_and_ctrl_end_reaches_document_end() {
    let line = "alpha beta gamma delta epsilon zeta eta theta";
    let source = format!("{line}\nlast line");
    let mut e = editor(&source, "navigation.txt", Mode::Edit);
    draw(&mut e, 22);
    e.handle(Event::Key(KeyEvent::new(K::End, M::NONE)));
    let end = e.buffer.cursors[0].head;
    assert_eq!(end, line.len());
    draw(&mut e, 22);
    e.handle(Event::Key(KeyEvent::new(K::Home, M::NONE)));
    assert!(e.buffer.cursors[0].head < end);
    e.handle(Event::Key(KeyEvent::new(K::End, M::NONE)));
    e.handle(Event::Key(KeyEvent::new(K::Down, M::NONE)));
    assert!(e.buffer.cursors[0].head > end);
    e.handle(Event::Key(KeyEvent::new(K::Up, M::NONE)));
    assert_eq!(e.buffer.cursors[0].head, end);
    e.handle(Event::Key(KeyEvent::new(K::End, M::CONTROL)));
    assert_eq!(e.buffer.cursors[0].head, source.len());
}

#[test]
fn end_and_shift_end_use_each_cursor_source_line_without_selecting_newlines() {
    use editio::buffer::Cursor;
    for newline in ["\n", "\r\n"] {
        for width in [16, 40] {
            for multiple in [false, true] {
                for select in [false, true] {
                    let lines = [
                        "日本語 alpha beta gamma delta epsilon zeta",
                        "    second long line with café and more words",
                        "",
                    ];
                    let source = lines.join(newline);
                    let mut e = editor(&source, "navigation.txt", Mode::Edit);
                    let starts = [2, lines[0].chars().count() + newline.chars().count() + 5];
                    e.buffer.cursors = starts[..if multiple { 2 } else { 1 }]
                        .iter()
                        .map(|&p| Cursor::at(p))
                        .collect();
                    draw(&mut e, width);
                    e.handle(Event::Key(KeyEvent::new(
                        K::End,
                        if select { M::SHIFT } else { M::NONE },
                    )));
                    for (i, cursor) in e.buffer.cursors.iter().enumerate() {
                        let end = e.buffer.text.line_to_char(i) + lines[i].chars().count();
                        assert_eq!(cursor.head, end);
                        assert_eq!(cursor.anchor, if select { starts[i] } else { end });
                    }
                    draw(&mut e, width);
                    let cursors = e.buffer.cursors.clone();
                    e.handle(Event::Key(KeyEvent::new(
                        K::End,
                        if select { M::SHIFT } else { M::NONE },
                    )));
                    assert_eq!(e.buffer.cursors, cursors);
                    assert_eq!(e.buffer.text.to_string(), source);
                    assert!(!e.buffer.dirty());
                }
            }
        }
    }
}

#[test]
fn markdown_tables_and_fences_remain_single_source_rows_and_scroll_horizontally() {
    let source = "| Long header that exceeds the window | Other header |\n| --- | --- |\n| A long cell that exceeds the window | Value |\n\n```rust\nlet very_long_variable_name = 123456789;\n```\n";
    let mut e = editor(source, "geometry.md", Mode::Edit);
    let screen = draw(&mut e, 25);
    assert!(text(&screen, 4, 25).starts_with("| --- | --- |"));
    assert!(text(&screen, 7, 25).starts_with("```rust"));
    assert!(text(&screen, 9, 25).starts_with("```"));
    // Reach the first source line's end without using document-end navigation.
    for _ in 0..source.lines().next().unwrap().chars().count() {
        e.handle(Event::Key(KeyEvent::new(K::Right, M::NONE)));
    }
    let scrolled = draw(&mut e, 25);
    assert!(text(&scrolled, 3, 25).contains("Other header |"));
    e.act(Action::SelectAll);
    assert_eq!(copy(&mut e), source);
}
