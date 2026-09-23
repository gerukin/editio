use crossterm::event::{
    Event, KeyCode as K, KeyEvent, KeyModifiers as M, MouseButton as B, MouseEvent,
    MouseEventKind as MK,
};
use editio::{Action, Editor, Mode, Outcome, buffer::Buffer};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer as Screen,
    layout::Rect,
    style::{Color, Modifier},
};
fn editor(source: &str, path: &str) -> Editor {
    let mut b = Buffer::new(source);
    b.path = Some(path.into());
    Editor::new(b)
}
fn draw(e: &mut Editor) -> Screen {
    let mut t = Terminal::new(TestBackend::new(90, 40)).unwrap();
    t.draw(|f| e.draw(f, Rect::new(4, 4, 80, 33))).unwrap();
    t.backend().buffer().clone()
}
fn ready(e: &mut Editor) -> Screen {
    draw(e);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    draw(e)
}
fn position(screen: &Screen, needle: &str) -> (u16, u16) {
    for y in 0..screen.area.height {
        let line: String = (0..screen.area.width)
            .map(|x| screen[(x, y)].symbol())
            .collect();
        if let Some(byte) = line.find(needle) {
            use unicode_width::UnicodeWidthStr;
            return (line[..byte].width() as u16, y);
        }
    }
    panic!("missing {needle}: {screen:?}")
}
fn mouse(e: &mut Editor, kind: MK, p: (u16, u16)) -> Outcome {
    e.handle(Event::Mouse(MouseEvent {
        kind,
        column: p.0,
        row: p.1,
        modifiers: M::NONE,
    }))
}
fn copy(e: &mut Editor) -> String {
    match e.act(Action::Copy) {
        Outcome::CopyRequested(s) => s,
        other => panic!("{other:?}"),
    }
}
#[test]
fn link_clicks_copy_hidden_urls_and_context_menu_copies_visible_text() {
    let mut e = editor(
        "# Resources\n\n[**Rust book**](https://doc.rust-lang.org/book/) and [Rust book](https://example.org/other).\n",
        "links.md",
    );
    let screen = ready(&mut e);
    let p = position(&screen, "Rust book");
    let all: String = screen.content.iter().map(|c| c.symbol()).collect();
    assert!(!all.contains("https://"));
    mouse(&mut e, MK::Down(B::Left), p);
    assert_eq!(
        mouse(&mut e, MK::Up(B::Left), p),
        Outcome::CopyRequested("https://doc.rust-lang.org/book/".into())
    );
    assert_eq!(e.mode, Mode::View);
    assert!(!e.source);
    mouse(&mut e, MK::Down(B::Right), p);
    let menu = draw(&mut e);
    let item = position(&menu, "Copy link text");
    assert_eq!(
        mouse(&mut e, MK::Down(B::Left), item),
        Outcome::CopyRequested("Rust book".into())
    );
    mouse(&mut e, MK::Down(B::Right), p);
    draw(&mut e);
    e.handle(Event::Key(KeyEvent::new(K::Down, M::NONE)));
    assert_eq!(
        e.handle(Event::Key(KeyEvent::new(K::Enter, M::NONE))),
        Outcome::CopyRequested("https://doc.rust-lang.org/book/".into())
    );
    assert!(!e.source);
}
#[test]
fn view_drag_copies_rendered_characters_and_keeps_mode_and_source() {
    let original = "# Title\n\nSelect **bold** text and 猫 here.\n";
    let mut e = editor(original, "selection.md");
    let screen = ready(&mut e);
    let p = position(&screen, "Select bold text");
    mouse(&mut e, MK::Down(B::Left), p);
    mouse(&mut e, MK::Drag(B::Left), (p.0 + 16, p.1));
    mouse(&mut e, MK::Up(B::Left), (p.0 + 16, p.1));
    assert_eq!(copy(&mut e), "Select bold text");
    assert!(!e.source);
    assert_eq!(e.mode, Mode::View);
    assert_eq!(e.buffer.text.to_string(), original);
    e.act(Action::SelectAll);
    assert!(copy(&mut e).contains("Select bold text"));
    assert!(!copy(&mut e).contains("**"));
    assert!(!e.source);
}
#[test]
fn table_selection_is_confined_to_cell_columns_and_excludes_borders() {
    for (name, source) in [
        ("table.csv", "Left,Right\nALPHA,BETA\nGAMMA,DELTA\n"),
        (
            "table.md",
            "| Left | Right |\n| --- | --- |\n| ALPHA | BETA |\n| GAMMA | DELTA |\n",
        ),
    ] {
        let mut e = editor(source, name);
        let screen = ready(&mut e);
        let p = position(&screen, "ALPHA");
        let q = position(&screen, "DELTA");
        mouse(&mut e, MK::Down(B::Left), p);
        mouse(&mut e, MK::Drag(B::Left), (q.0 + 5, q.1));
        let selected = copy(&mut e);
        assert!(selected.contains("ALPHA"), "{selected}");
        assert!(
            !selected.contains("BETA") && !selected.contains("DELTA") && !selected.contains('│'),
            "{selected}"
        );
        assert!(!e.source);
        assert_eq!(e.mode, Mode::View);
    }
}
#[test]
fn headings_are_styled_and_mermaid_has_no_visible_fence() {
    let mut e = editor(
        "# Heading\n\nBody text\n\n```mermaid\nflowchart TD\nA[Open] --> B[Close]\n```\n",
        "heading.md",
    );
    let screen = ready(&mut e);
    let p = position(&screen, "Heading");
    assert!(screen[p].modifier.contains(Modifier::BOLD));
    assert_eq!(screen[p].fg, Color::Magenta);
    let all: String = screen.content.iter().map(|c| c.symbol()).collect();
    assert!(!all.contains("```"));
    assert!(!all.contains("flowchart TD"));
    assert!(all.contains("Open") && all.contains("Close"));
    let p = position(&screen, "Open");
    let q = position(&screen, "Close");
    mouse(&mut e, MK::Down(B::Left), p);
    mouse(&mut e, MK::Drag(B::Left), (q.0 + 5, q.1));
    assert!(!copy(&mut e).contains("Close"));
}
#[test]
fn plain_text_view_clicks_remain_read_only() {
    let mut e = editor("hello world", "plain.txt");
    let screen = ready(&mut e);
    let p = position(&screen, "hello");
    mouse(&mut e, MK::Down(B::Left), p);
    mouse(&mut e, MK::Drag(B::Left), (p.0 + 5, p.1));
    assert_eq!(copy(&mut e), "hello");
    assert_eq!(e.mode, Mode::View);
}

#[test]
fn wrapped_unicode_links_and_table_links_keep_their_targets() {
    let source = format!(
        "# Links\n\n[{}尾](https://example.org/wrapped)\n\n| Link | Other |\n| --- | --- |\n| [Table target](https://example.org/table) | text |\n",
        "word ".repeat(20)
    );
    let mut e = editor(&source, "wrapped.md");
    let screen = ready(&mut e);
    let p = position(&screen, "尾");
    // Both terminal cells of a wide character should target the same link.
    mouse(&mut e, MK::Down(B::Left), (p.0 + 1, p.1));
    assert_eq!(
        mouse(&mut e, MK::Up(B::Left), (p.0 + 1, p.1)),
        Outcome::CopyRequested("https://example.org/wrapped".into())
    );
    e.scroll = 2;
    let screen = draw(&mut e);
    let p = position(&screen, "Table target");
    mouse(&mut e, MK::Down(B::Left), p);
    assert_eq!(
        mouse(&mut e, MK::Up(B::Left), p),
        Outcome::CopyRequested("https://example.org/table".into())
    );
    mouse(&mut e, MK::Down(B::Right), p);
    draw(&mut e);
    e.handle(Event::Key(KeyEvent::new(K::Esc, M::NONE)));
    let screen = draw(&mut e);
    assert!(
        !screen
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
            .contains("Copy URL")
    );
    assert!(!e.source);
}

fn screen_text(screen: &Screen) -> String {
    screen.content.iter().map(|c| c.symbol()).collect()
}
fn click(e: &mut Editor, p: (u16, u16)) -> Outcome {
    mouse(e, MK::Down(B::Left), p);
    mouse(e, MK::Up(B::Left), p)
}
#[test]
fn double_click_selects_rendered_words_including_link_words() {
    for (source, needle) in [
        ("Plain words here", "words"),
        ("[Linked words here](https://example.org)", "words"),
    ] {
        let mut e = editor(source, "double.md");
        let screen = ready(&mut e);
        let p = position(&screen, needle);
        click(&mut e, (p.0 + 2, p.1));
        assert_eq!(click(&mut e, (p.0 + 2, p.1)), Outcome::Handled);
        assert_eq!(copy(&mut e), "words");
        assert!(!e.source);
        assert_eq!(e.mode, Mode::View);
    }
}
#[test]
fn heading_text_copies_relative_and_absolute_paths_and_text() {
    let mut e = editor(
        "## Hello, **World**!\n\ntext\n\n## Hello, **World**!\n",
        "docs/note.md",
    );
    let screen = ready(&mut e);
    let p = position(&screen, "Hello, World!");
    let anchor = p;
    assert_eq!(screen[anchor].symbol(), "H");
    assert_eq!(screen[anchor].fg, Color::Cyan);
    assert!(screen[anchor].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(
        click(&mut e, anchor),
        Outcome::CopyRequested("docs/note.md#hello-world".into())
    );
    mouse(&mut e, MK::Down(B::Right), anchor);
    let menu = draw(&mut e);
    let item = position(&menu, "Copy absolute path");
    let absolute = std::env::current_dir().unwrap().join("docs/note.md");
    assert_eq!(
        mouse(&mut e, MK::Down(B::Left), item),
        Outcome::CopyRequested(format!("{}#hello-world", absolute.display()))
    );
    mouse(&mut e, MK::Down(B::Right), anchor);
    let menu = draw(&mut e);
    let item = position(&menu, "Copy heading text");
    assert_eq!(
        mouse(&mut e, MK::Down(B::Left), item),
        Outcome::CopyRequested("Hello, World!".into())
    );
    let second = (
        anchor.0,
        (p.1 + 1..36)
            .find(|&y| screen[(p.0, y)].symbol() == "H")
            .unwrap(),
    );
    assert_eq!(
        click(&mut e, second),
        Outcome::CopyRequested("docs/note.md#hello-world-1".into())
    );
}
#[test]
fn details_defaults_toggle_nested_state_and_comments_are_ignored() {
    let source = "# Page\n\n<!-- SECRET COMMENT -->\n\n<details>\n<summary>Hidden panel</summary>\n\nHidden body\n\n<details open>\n<summary>Child panel</summary>\n\nChild body\n\n</details>\n</details>\n\n<details open=\"false\">\n<summary>Open panel</summary>\n\nDefault body\n\n</details>\n\n`<!-- literal -->`\n";
    let mut e = editor(source, "details.md");
    let screen = ready(&mut e);
    let text = screen_text(&screen);
    assert!(!text.contains("SECRET COMMENT"));
    assert!(!text.contains("Hidden body"));
    assert!(text.contains("Default body"));
    assert!(text.contains("<!-- literal -->"));
    click(&mut e, position(&screen, "Hidden panel"));
    let screen = ready(&mut e);
    assert!(screen_text(&screen).contains("Hidden body"));
    assert!(screen_text(&screen).contains("Child body"));
    click(&mut e, position(&screen, "Child panel"));
    let screen = ready(&mut e);
    assert!(!screen_text(&screen).contains("Child body"));
    click(&mut e, position(&screen, "Hidden panel"));
    let screen = ready(&mut e);
    click(&mut e, position(&screen, "Hidden panel"));
    let screen = ready(&mut e);
    assert!(!screen_text(&screen).contains("Child body"));
    assert!(screen_text(&screen).contains("Hidden body"));
    assert_eq!(e.mode, Mode::View);
    assert!(!e.source);
    assert_eq!(e.buffer.text.to_string(), source);
}
#[test]
fn callout_border_is_colored_body_is_normal_and_padding_is_not_copied() {
    let mut e = editor(
        "> [!WARNING]\n> Normal body with **bold** text.\n>\n> ```rust\n> let n = 42;\n> ```\n",
        "callout.md",
    );
    let screen = ready(&mut e);
    let body = position(&screen, "Normal body");
    assert_eq!(screen[(4, body.1)].symbol(), "┃");
    assert_eq!(screen[(4, body.1)].fg, Color::Yellow);
    assert_eq!(screen[body].fg, Color::Reset);
    assert!(
        screen[position(&screen, "bold")]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(screen[position(&screen, "let")].fg, Color::Magenta);
    mouse(&mut e, MK::Down(B::Left), (4, body.1));
    mouse(&mut e, MK::Drag(B::Left), (body.0 + 11, body.1));
    assert_eq!(copy(&mut e), "Normal body");
    let selected = draw(&mut e);
    assert!(!selected[(4, body.1)].modifier.contains(Modifier::REVERSED));
    let mut table = editor("A,B\nshort,longword\n", "padding.csv");
    let screen = ready(&mut table);
    let p = position(&screen, "short");
    mouse(&mut table, MK::Down(B::Left), (p.0 - 1, p.1));
    mouse(&mut table, MK::Drag(B::Left), (p.0 + 6, p.1));
    assert_eq!(copy(&mut table), "short");
}
#[test]
fn rule_reflows_and_only_one_bottom_bar_is_drawn() {
    let mut e = editor("before\n\n---\n\nafter\n", "rule.md");
    for width in [32, 64] {
        let mut t = Terminal::new(TestBackend::new(width, 12)).unwrap();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while e.has_background_work() {
            assert!(std::time::Instant::now() < deadline);
            e.poll_background();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let b = t.backend().buffer();
        let row = (1..11).find(|&y| b[(0, y)].symbol() == "─").unwrap();
        assert!(
            (0..width).all(|x| b[(x, row)].symbol() == "─" && b[(x, row)].fg == Color::DarkGray)
        );
        assert!(
            !b[(0, 11)]
                .modifier
                .intersects(Modifier::REVERSED | Modifier::DIM)
        );
        assert_eq!(b[(0, 11)].fg, Color::Black);
        assert_eq!(b[(0, 11)].bg, Color::Blue);
        assert_eq!(b[(0, 10)].bg, Color::Reset);
        assert!(!b[(0, 10)].modifier.contains(Modifier::REVERSED));
    }
}

#[test]
fn details_support_keyboard_and_keep_state_on_resize() {
    let mut e = editor(
        "<details>\n<summary>Panel</summary>\n\nVisible after expansion\n\n</details>\n",
        "keyboard.md",
    );
    ready(&mut e);
    e.handle(Event::Key(KeyEvent::new(K::Tab, M::NONE)));
    e.handle(Event::Key(KeyEvent::new(K::Enter, M::NONE)));
    assert!(screen_text(&ready(&mut e)).contains("Visible after expansion"));
    let mut t = Terminal::new(TestBackend::new(60, 20)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    t.draw(|f| e.draw(f, f.area())).unwrap();
    assert!(screen_text(t.backend().buffer()).contains("Visible after expansion"));
    assert_eq!(e.mode, Mode::View);
}
#[test]
fn wrapped_callout_lines_keep_the_border_and_do_not_copy_padding() {
    for (intro, border) in [("> [!NOTE]\n", "┃"), ("", "│")] {
        let mut e = editor(
            &format!(
                "{intro}> This is a long paragraph that must wrap onto several lines while keeping a border on every line and normal text coloring.\n"
            ),
            "wrap.md",
        );
        let mut t = Terminal::new(TestBackend::new(32, 16)).unwrap();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while e.has_background_work() {
            assert!(std::time::Instant::now() < deadline);
            e.poll_background();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let b = t.backend().buffer();
        let start = position(b, "This is");
        let end = position(b, "coloring.");
        for y in start.1..=end.1 {
            assert_eq!(b[(0, y)].symbol(), border);
            assert_eq!(b[(2, y)].modifier.contains(Modifier::DIM), border == "│");
        }
        mouse(&mut e, MK::Down(B::Left), (0, start.1));
        mouse(&mut e, MK::Drag(B::Left), (end.0 + 9, end.1));
        let text = copy(&mut e);
        assert!(!text.contains(['┃', '│']));
        assert!(text.lines().all(|line| !line.starts_with(' ')), "{text:?}");
    }
}

#[test]
fn empty_markdown_selection_is_safe_and_full_table_copy_omits_borders() {
    let mut e = editor("", "empty.md");
    ready(&mut e);
    e.act(Action::SelectAll);
    assert_eq!(copy(&mut e), "");
    let mut table = editor("A,B\nshort,longword\n", "plain-copy.csv");
    ready(&mut table);
    table.act(Action::SelectAll);
    let selected = copy(&mut table);
    assert!(!selected.contains('│') && !selected.contains('─'));
    assert!(selected.contains("short\tlongword"), "{selected:?}");
}

#[test]
fn heading_markers_and_gaps_are_not_underlined_at_any_level() {
    let source = (1..=6)
        .map(|level| format!("{} Heading{level}\n\n", "#".repeat(level)))
        .collect::<String>();
    let mut e = editor(&source, "headings.md");
    let screen = ready(&mut e);
    for level in 2..=6u16 {
        let (x, y) = position(&screen, &format!("Heading{level}"));
        assert_eq!(x, 6);
        assert_eq!(
            screen[(x - 2, y)].symbol(),
            if level == 2 {
                "─".to_owned()
            } else {
                level.to_string()
            }
        );
        assert!(!screen[(x - 2, y)].modifier.contains(Modifier::REVERSED));
        if level >= 3 {
            assert_eq!(screen[(x - 2, y)].bg, Color::Reset);
        }
        assert_eq!(
            screen[(x - 2, y)].modifier.contains(Modifier::DIM),
            level >= 3
        );
        assert_eq!(screen[(x - 2, y)].fg, Color::DarkGray);
        assert!(!screen[(x - 2, y)].modifier.contains(Modifier::BOLD));
        assert!(!screen[(x - 2, y)].modifier.contains(Modifier::UNDERLINED));
        for column in [x - 1, x + 8] {
            assert_eq!(screen[(column, y)].symbol(), " ");
            assert!(!screen[(column, y)].modifier.contains(Modifier::UNDERLINED));
        }
        for column in x..x + 8 {
            assert_eq!(screen[(column, y)].fg, Color::Cyan);
            assert!(screen[(column, y)].modifier.contains(Modifier::UNDERLINED));
        }
        assert_eq!(
            screen[(x + 9, y)].symbol(),
            if level >= 4 { " " } else { "─" }
        );
        if level < 4 {
            assert_eq!(screen[(x + 9, y)].fg, Color::DarkGray);
        }
        assert!(!screen[(x + 9, y)].modifier.contains(Modifier::UNDERLINED));
    }
}

#[test]
fn quotes_dim_nested_markdown_and_copy_without_borders() {
    let mut e = editor(
        "> Normal **bold** and [link](https://example.com).\n>\n> ### Subheading\n>\n> ```rust\n> let answer = 42;\n> ```\n>\n> | Key | Value |\n> | --- | --- |\n> | item | result |\n>\n> > Nested words\n>\n> > [!NOTE]\n> > Alert body\n\nOutside\n",
        "quotes.md",
    );
    let screen = ready(&mut e);
    for word in [
        "Normal",
        "bold",
        "link",
        "Subheading",
        "let",
        "item",
        "Nested words",
        "Alert body",
    ] {
        assert!(
            screen[position(&screen, word)]
                .modifier
                .contains(Modifier::DIM),
            "{word}"
        );
    }
    assert_eq!(screen[position(&screen, "Normal")].fg, Color::Reset);
    assert!(
        screen[position(&screen, "bold")]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(screen[position(&screen, "let")].fg, Color::Magenta);
    assert!(
        !screen[position(&screen, "Outside")]
            .modifier
            .contains(Modifier::DIM)
    );
    let p = position(&screen, "Normal");
    assert_eq!(screen[(p.0 - 2, p.1)].symbol(), "│");
    assert_eq!(screen[(p.0 - 2, p.1)].fg, Color::Green);
    e.act(Action::SelectAll);
    let text = copy(&mut e);
    assert!(text.contains("Nested words"), "{text:?}");
    assert!(text.contains("item\tresult"), "{text:?}");
    assert!(!text.contains(['│', '┃', '>']), "{text:?}");
}

#[test]
fn quoted_details_keep_their_border_and_muted_contents() {
    let mut e = editor(
        "> <details open>\n> <summary>Quoted panel</summary>\n>\n> **Inside panel**\n>\n> </details>\n\nOutside panel\n",
        "quoted-details.md",
    );
    let screen = ready(&mut e);
    for text in ["Quoted panel", "Inside panel"] {
        let p = position(&screen, text);
        assert!(screen[p].modifier.contains(Modifier::DIM));
        assert_eq!(screen[(4, p.1)].symbol(), "│");
    }
    assert!(
        !screen[position(&screen, "Outside panel")]
            .modifier
            .contains(Modifier::DIM)
    );
    click(&mut e, position(&screen, "Quoted panel"));
    assert!(!screen_text(&ready(&mut e)).contains("Inside panel"));
}

#[test]
fn markdown_edit_palette_matches_preview_without_changing_source_characters() {
    let source = "### Héading\n\nA **bold** *italic* ~~removed~~ [link](https://example.com) and `inline`.\n\n> Quoted **words**\n\n> [!WARNING]\n> Alert body\n>\n> ```rust\n> let number = 42;\n> ```\n\n---\n";
    let mut e = editor(source, "source-colors.md");
    let view = ready(&mut e);
    e.mode = Mode::Edit;
    let edit = draw(&mut e);
    for word in [
        "Héading",
        "bold",
        "italic",
        "removed",
        "link",
        "inline",
        "Quoted",
        "words",
        "WARNING",
        "Alert body",
        "let",
        "42",
    ] {
        let preview_word = if word == "WARNING" { "Warning" } else { word };
        let a = &edit[position(&edit, word)];
        let b = &view[position(&view, preview_word)];
        assert_eq!(a.fg, b.fg, "{word}");
        assert_eq!(a.modifier, b.modifier, "{word}");
    }
    assert_eq!(edit[position(&edit, "---")].fg, Color::DarkGray);
    assert!(
        !edit[position(&edit, "### ")]
            .modifier
            .contains(Modifier::UNDERLINED)
    );
    for line in source.lines().filter(|s| !s.is_empty()) {
        position(&edit, line);
    }
    assert_eq!(e.buffer.text.to_string(), source);
    // Starting inside a callout's fence preserves the callout context and code colors.
    e.scroll = 10;
    let scrolled = draw(&mut e);
    let keyword = &scrolled[position(&scrolled, "let")];
    assert_eq!(keyword.fg, Color::Magenta);
    assert!(!keyword.modifier.contains(Modifier::DIM));
}

#[test]
fn markdown_source_tables_preserve_structure_and_inline_styles_when_scrolled() {
    let source = "Intro\n\n| Name | *Status* |\n| :--- | ---: |\n| **Bold cell** | [Reference][target] |\n| `a\\|b` | ~~Removed cell~~ |\n\n> | Nested | Data |\n> | --- | --- |\n> | Quoted cell | `quoted code` |\n\n[target]: https://example.com\n\nAfter table\n";
    let mut e = editor(source, "tables.md");
    let view = ready(&mut e);
    e.mode = Mode::Edit;
    let edit = draw(&mut e);
    for text in [
        "Name",
        "Status",
        "Bold cell",
        "Reference",
        "Removed cell",
        "Nested",
        "Quoted cell",
        "quoted code",
    ] {
        let a = &edit[position(&edit, text)];
        let b = &view[position(&view, text)];
        assert_eq!(a.fg, b.fg, "{text}");
        assert_eq!(a.modifier, b.modifier, "{text}");
    }
    for text in ["| :--- | ---: |", ":---", "---:", "| Name"] {
        assert_eq!(edit[position(&edit, text)].fg, Color::DarkGray, "{text}");
    }
    assert_eq!(edit[position(&edit, "a\\|b")].fg, Color::Yellow);
    let pipe = position(&edit, "a\\|b");
    assert_eq!(edit[(pipe.0 + 2, pipe.1)].fg, Color::Yellow);
    assert_eq!(edit[position(&edit, "> | --- | --- |")].fg, Color::Green);
    for line in source.lines().filter(|s| !s.is_empty()) {
        position(&edit, line);
    }
    e.scroll = 4;
    let scrolled = draw(&mut e);
    assert_eq!(scrolled[position(&scrolled, "Reference")].fg, Color::Blue);
    assert!(
        scrolled[position(&scrolled, "Quoted cell")]
            .modifier
            .contains(Modifier::DIM)
    );
    assert_eq!(e.buffer.text.to_string(), source);
    e.act(Action::SelectAll);
    e.handle(Event::Paste(
        "Name | Status\nThis is prose, without a table separator.\n".into(),
    ));
    e.scroll = 0;
    let changed = draw(&mut e);
    assert_eq!(changed[position(&changed, "Name")].fg, Color::Reset);
    e.act(Action::Undo);
    e.scroll = 0;
    let restored = draw(&mut e);
    assert_eq!(restored[position(&restored, "Name")].fg, Color::Cyan);
}

#[test]
fn heading_text_keeps_actions_between_rules_at_every_width() {
    fn sized(e: &mut Editor, width: u16) -> Screen {
        let mut t = Terminal::new(TestBackend::new(width, 30)).unwrap();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while e.has_background_work() {
            assert!(std::time::Instant::now() < deadline);
            e.poll_background();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        t.draw(|f| e.draw(f, f.area())).unwrap();
        t.backend().buffer().clone()
    }
    for width in [24, 59, 60, 80] {
        let mut e = editor("## Hello **World**\n\ntext", "docs/note.md");
        let s = sized(&mut e, width);
        let row: String = (0..width).map(|x| s[(x, 0)].symbol()).collect();
        assert_eq!(
            row.trim_end(),
            format!("─ Hello World {}", "─".repeat(width as usize - 14))
        );
        for p in [(3, 0), (6, 0), (10, 0)] {
            // Reset only the timed double-click tracking by using a fresh editor.
            let mut e = editor("## Hello **World**\n\ntext", "docs/note.md");
            sized(&mut e, width);
            assert_eq!(
                click(&mut e, p),
                Outcome::CopyRequested("docs/note.md#hello-world".into())
            );
            mouse(&mut e, MK::Down(B::Right), p);
            let menu = sized(&mut e, width);
            assert!(screen_text(&menu).contains("Copy heading text"));
            let target = position(&menu, "Copy heading text");
            assert_eq!(
                mouse(&mut e, MK::Down(B::Left), target),
                Outcome::CopyRequested("Hello World".into())
            );
        }
        // Resize both ways: the heading remains selectable and double-clickable.
        let mut e = editor("## Hello **World**\n", "docs/note.md");
        sized(&mut e, 80);
        sized(&mut e, width);
        click(&mut e, (4, 0));
        assert_eq!(click(&mut e, (4, 0)), Outcome::Handled);
        assert_eq!(copy(&mut e), "Hello");
    }
}

#[test]
fn centered_h1_has_muted_rules_and_preserves_heading_actions() {
    let mut e = editor("# Hello **World**\n\nBody", "docs/title.md");
    let screen = ready(&mut e);
    let p = position(&screen, "Hello World");
    assert_eq!(p, (4 + (80 - 11) / 2, 4));
    assert_eq!(screen[(4, 4)].symbol(), "━");
    assert_eq!(screen[(83, 4)].symbol(), "━");
    assert_eq!(screen[(4, 4)].fg, Color::DarkGray);
    assert!(!screen[(4, 4)].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(screen[p].fg, Color::Magenta);
    assert!(screen[p].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(
        click(&mut e, p),
        Outcome::CopyRequested("docs/title.md#hello-world".into())
    );
    mouse(&mut e, MK::Down(B::Right), p);
    let menu = draw(&mut e);
    let target = position(&menu, "Copy heading text");
    assert_eq!(
        mouse(&mut e, MK::Down(B::Left), target),
        Outcome::CopyRequested("Hello World".into())
    );
    e.act(Action::SelectAll);
    assert_eq!(copy(&mut e), "Hello World\n\nBody");
}

#[test]
fn frontmatter_disclosure_uses_yaml_code_and_preserves_source() {
    let source = "---\nname: demo\nenabled: true\n---\n\n# Body\n\n<details open>\n<summary>Other</summary>\n\nVisible body\n\n</details>\n";
    let mut e = editor(source, "frontmatter.md");
    let closed = ready(&mut e);
    let label = position(&closed, "Frontmatter");
    assert_eq!(closed[label].fg, Color::DarkGray);
    assert!(closed[label].modifier.contains(Modifier::UNDERLINED));
    for x in label.0 - 2..label.0 {
        assert_eq!(closed[(x, label.1)].fg, Color::DarkGray);
        assert!(!closed[(x, label.1)].modifier.contains(Modifier::UNDERLINED));
    }
    assert!(!screen_text(&closed).contains("name: demo"));
    assert!(screen_text(&closed).contains("Visible body"));
    click(&mut e, position(&closed, "Frontmatter"));
    let open = ready(&mut e);
    let name = position(&open, "name:");
    assert_ne!(open[name].fg, Color::Reset);
    assert!(screen_text(&open).contains("YAML"));
    let button = position(&open, "⧉ Copy");
    assert_eq!(
        click(&mut e, button),
        Outcome::CopyRequested("name: demo\nenabled: true\n".into())
    );
    click(&mut e, position(&open, "Frontmatter"));
    let closed = ready(&mut e);
    assert!(!screen_text(&closed).contains("name: demo"));
    e.act(Action::ToggleMode);
    assert_eq!(e.mode, Mode::Edit);
    assert_eq!(e.buffer.text.to_string(), source);
}

#[test]
fn heading_rules_and_quote_borders_do_not_inherit_text_underlining() {
    let mut e = editor(
        "> ## A heading inside a quote also aligns its wrapped title text while retaining the quote border and its muted treatment\n",
        "quote-heading.md",
    );
    let screen = ready(&mut e);
    let p = position(&screen, "A heading");
    assert!(screen[p].modifier.contains(Modifier::UNDERLINED));
    for y in p.1..p.1 + 2 {
        assert_eq!(screen[(4, y)].symbol(), "│");
        for x in 4..p.0 {
            assert!(
                !screen[(x, y)].modifier.contains(Modifier::UNDERLINED),
                "cell ({x},{y})"
            );
            assert!(!screen[(x, y)].modifier.contains(Modifier::CROSSED_OUT));
        }
    }
}

#[test]
fn heading_surfaces_match_in_view_and_edit_without_changing_source() {
    use tapp_ui::theme::{HEADING_CYAN, HEADING_MAGENTA, Theme};
    let source = "# First\n\n## Second\n\n### Third\n";
    let mut e = editor(source, "heading-colors.md");
    e.theme = Theme::TokyoNightOmarchy;
    for mode in [Mode::View, Mode::Edit] {
        e.mode = mode;
        let screen = ready(&mut e);
        for (title, color, bg) in [
            ("First", Color::Magenta, HEADING_MAGENTA),
            ("Second", Color::Cyan, HEADING_CYAN),
            ("Third", Color::Cyan, Color::Reset),
        ] {
            let p = position(&screen, title);
            assert_eq!(screen[p].fg, e.theme.color(color, false));
            assert_eq!(screen[p].bg, e.theme.color(bg, true));
            assert_eq!(screen[(83, p.1)].bg, e.theme.color(bg, true));
        }
        if mode == Mode::View {
            let (x, y) = position(&screen, "Second");
            let digit = &screen[(x - 2, y)];
            assert_eq!(digit.symbol(), "─");
            assert_eq!(digit.style(), screen[(83, y)].style());
            assert_eq!(digit.fg, e.theme.color(Color::DarkGray, false));
            assert_eq!(digit.bg, e.theme.color(HEADING_CYAN, true));
            assert!(
                !digit
                    .modifier
                    .intersects(Modifier::DIM | Modifier::BOLD | Modifier::REVERSED)
            );
        }
        assert_eq!(e.buffer.text.to_string(), source);
    }
}

#[test]
fn opt_in_link_menu_offers_inspect_and_local_open_without_changing_copy() {
    let mut e = editor(
        "[local](notes/plan.md) and [web](https://example.com/path).",
        "links.md",
    );
    e.file_navigation = true;
    let s = ready(&mut e);
    let p = position(&s, "local");
    mouse(&mut e, MK::Down(B::Right), p);
    let s = draw(&mut e);
    position(&s, "See full URL");
    position(&s, "Open in Editio");
    for _ in 0..4 {
        e.handle(Event::Key(KeyEvent::new(K::Down, M::NONE)));
    }
    assert_eq!(
        e.handle(Event::Key(KeyEvent::new(K::Enter, M::NONE))),
        Outcome::LinkRequested {
            url: "notes/plan.md".into(),
            action: 2
        }
    );
}

#[test]
fn urls_mute_separators_and_keep_link_and_code_base_colors() {
    let mut e = editor(
        "<https://example.com/path>\n\n`file:///tmp/note.md`",
        "urls.md",
    );
    let s = ready(&mut e);
    let p = position(&s, "https:");
    assert_eq!(s[p].fg, Color::Blue);
    assert_eq!(s[(p.0 + 6, p.1)].fg, Color::DarkGray);
    let p = position(&s, "file:");
    assert_eq!(s[p].fg, Color::Yellow);
    assert_eq!(s[(p.0 + 5, p.1)].fg, Color::DarkGray);
}
