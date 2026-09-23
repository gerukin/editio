use crossterm::event::{Event, KeyCode as K, KeyEvent, KeyModifiers as M};
use editio::{Action, Editor, Mode, buffer::Buffer};
use ratatui::{Terminal, backend::TestBackend};
fn key(e: &mut Editor, code: K, modifiers: M) {
    e.handle(Event::Key(KeyEvent::new(code, modifiers)));
}
fn draw(e: &mut Editor, width: u16, height: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    let b = t.backend().buffer();
    (0..height)
        .map(|y| (0..width).map(|x| b[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn preview(e: &mut Editor, width: u16, height: u16) -> String {
    draw(e, width, height);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    draw(e, width, height)
}
#[test]
fn tables_pin_scroll_resize_and_return_to_source_editing() {
    for extension in ["CSV", "tsv"] {
        let delimiter = if extension == "CSV" { ',' } else { '\t' };
        let original = format!("Name{delimiter}Description{delimiter}LastColumn\n")
            + &(0..30)
                .map(|i| format!("item{i}{delimiter}long description here{delimiter}value{i}\n"))
                .collect::<String>();
        let mut b = Buffer::new(&original);
        b.path = Some(format!("table.{extension}").into());
        let mut e = Editor::new(b);
        assert!(e.rendered_view());
        let initial = preview(&mut e, 25, 12);
        assert!(!initial.contains('│'));
        assert!(!initial.contains(['┌', '┐', '└', '┘']));
        key(&mut e, K::PageDown, M::NONE);
        let screen = draw(&mut e, 25, 12);
        assert!(screen.contains("Name"));
        assert!(e.scroll > 0);
        // Forty one-column presses reach the formerly ten four-column steps.
        for _ in 0..40 {
            key(&mut e, K::Right, M::NONE);
        }
        assert!(draw(&mut e, 25, 12).contains("Last"));
        assert!(e.horizontal > 0);
        preview(&mut e, 80, 20);
        assert_eq!(e.horizontal, 0);
        e.act(Action::ToggleSource);
        assert!(!e.rendered_view());
        assert_eq!(e.buffer.text.to_string(), original);
        e.act(Action::ToggleMode);
        assert_eq!(e.mode, Mode::Edit);
        e.handle(Event::Paste("Edited".into()));
        e.act(Action::ToggleMode);
        e.act(Action::ToggleSource);
        assert!(preview(&mut e, 80, 20).contains("EditedName"));
    }
}
#[test]
fn search_switches_keep_input_and_apply_to_next_previous() {
    let mut e = Editor::new(Buffer::new("CAT cat c---t cat.txt"));
    e.act(Action::Find);
    e.handle(Event::Paste("cat".into()));
    key(&mut e, K::Tab, M::NONE);
    assert!(e.search_options.case_sensitive);
    key(&mut e, K::Tab, M::NONE);
    assert!(!e.search_options.case_sensitive);
    assert_eq!(e.prompt.as_ref().unwrap().value, "cat");
    assert!(draw(&mut e, 80, 20).contains("case:insensitive"));
    key(&mut e, K::Enter, M::NONE);
    assert_eq!(e.buffer.cursors[0].range(), 0..3);
    key(&mut e, K::Tab, M::NONE);
    assert_eq!(e.buffer.cursors[0].range(), 4..7);
    key(&mut e, K::BackTab, M::SHIFT);
    assert_eq!(e.buffer.cursors[0].range(), 0..3);
    e.act(Action::Find);
    key(&mut e, K::Char('u'), M::CONTROL);
    e.handle(Event::Paste("c?t".into()));
    key(&mut e, K::Char('w'), M::CONTROL);
    key(&mut e, K::BackTab, M::SHIFT);
    assert!(e.search_options.wildcards && e.search_options.fuzzy);
    key(&mut e, K::Enter, M::NONE);
    assert!(!e.message.contains("No matches"));
    e.act(Action::Find);
    assert!(e.search_options.wildcards && e.search_options.fuzzy);
    key(&mut e, K::Esc, M::NONE);
    assert!(e.prompt.is_none());
}

#[test]
fn view_arrows_scroll_one_column_without_changing_edit_navigation() {
    let mut e = Editor::new(Buffer::new(&"0123456789".repeat(30)));
    e.mode = Mode::View;
    e.wrap = false;
    draw(&mut e, 40, 10);
    key(&mut e, K::Right, M::NONE);
    assert_eq!(e.horizontal, 1);
    key(&mut e, K::Right, M::NONE);
    assert_eq!(e.horizontal, 2);
    key(&mut e, K::Left, M::NONE);
    assert_eq!(e.horizontal, 1);
    key(&mut e, K::Left, M::NONE);
    key(&mut e, K::Left, M::NONE);
    assert_eq!(e.horizontal, 0);
    e.mode = Mode::Edit;
    key(&mut e, K::Right, M::NONE);
    assert_eq!(e.horizontal, 0);
}
