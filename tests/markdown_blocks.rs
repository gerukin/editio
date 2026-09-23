use crossterm::event::{
    Event, KeyModifiers as M, MouseButton as B, MouseEvent, MouseEventKind as MK,
};
use editio::{Action, Editor, Mode, Outcome, buffer::Buffer};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer as Screen,
    layout::Rect,
    style::{Color, Modifier},
};
fn editor(source: &str, file: &str, mode: Mode) -> Editor {
    let mut b = Buffer::new(source);
    b.path = Some(file.into());
    let mut e = Editor::new(b);
    e.mode = mode;
    e
}
fn draw(e: &mut Editor, width: u16) -> Screen {
    let mut t = Terminal::new(TestBackend::new(width + 4, 80)).unwrap();
    t.draw(|f| e.draw(f, Rect::new(2, 2, width, 75))).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(2));
        t.draw(|f| e.draw(f, Rect::new(2, 2, width, 75))).unwrap();
    }
    t.backend().buffer().clone()
}
fn position(s: &Screen, text: &str) -> (u16, u16) {
    for y in 2..76 {
        let line: String = (2..s.area.width - 2).map(|x| s[(x, y)].symbol()).collect();
        if let Some(at) = line.find(text) {
            use unicode_width::UnicodeWidthStr;
            return (2 + line[..at].width() as u16, y);
        }
    }
    panic!("missing {text}: {s:?}")
}
fn all(e: &mut Editor) -> String {
    e.act(Action::SelectAll);
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
fn explicit_diff_fences_keep_both_sides_syntax_copy_without_change_borders() {
    let body = "-let old = 42;\n+let new = 43;\n unchanged\n";
    for fence in ["diff", "patch", "udiff", "diff-rust"] {
        let mut e = editor(
            &format!("# Patch\n\n```{fence}\n{body}```\n"),
            "diffs.md",
            Mode::View,
        );
        let s = draw(&mut e, 60);
        assert!(!e.is_diff(), "fences must not activate a comparison worker");
        assert_eq!(s[position(&s, "-let")].fg, Color::Red);
        assert_eq!(s[position(&s, "+let")].fg, Color::Green);
        assert_ne!(s[(61, position(&s, "-let").1)].symbol(), "┃");
        assert_ne!(s[(61, position(&s, "+let").1)].symbol(), "┃");
        if fence == "diff-rust" {
            let mut plain = editor("let new = 43;", "test.rs", Mode::Edit);
            let reference = draw(&mut plain, 60);
            assert_eq!(
                s[position(&s, "43")].fg,
                reference[position(&reference, "43")].fg
            );
        }
        assert!(all(&mut e).contains(body.trim_end_matches('\n')));
    }
    let mut ordinary = editor("```text\n-old\n+new\n```", "ordinary.md", Mode::View);
    let s = draw(&mut ordinary, 40);
    assert!(!ordinary.is_diff());
    assert_ne!(s[position(&s, "-old")].fg, Color::Red);
    let long = "+This very long line should wrap across multiple rows while retaining its border and keeping all original text.";
    let mut wrapped = editor(
        &format!("> ```diff-text\n> {long}\n> ```"),
        "wrapped.md",
        Mode::View,
    );
    let s = draw(&mut wrapped, 28);
    assert!((2..76).all(|y| s[(29, y)].symbol() != "┃"));
    assert!(all(&mut wrapped).contains(long));
    let mut untitled = Editor::new(Buffer::new("```diff\n-old\n+new\n```"));
    untitled.mode = Mode::View;
    draw(&mut untitled, 40);
    assert!(untitled.is_markdown());
    assert!(!untitled.is_diff());
}
#[test]
fn diff_fence_edit_highlighting_keeps_independent_contexts_and_source() {
    let body = "-/* old comment\n+let new = 43;\n-old comment */\n+let next = 44;\n";
    let source = format!("```diff-rust\n{body}```\n");
    let mut e = editor(&source, "diffs.md", Mode::Edit);
    for mode in [Mode::Edit, Mode::View, Mode::Edit] {
        e.mode = mode;
        let screen = draw(&mut e, 60);
        assert_eq!(screen[position(&screen, "+let")].fg, Color::Green);
        assert_eq!(screen[position(&screen, "-/*")].fg, Color::Red);
        for token in ["43", "44"] {
            assert_eq!(screen[position(&screen, token)].fg, Color::Yellow);
        }
    }
    assert_eq!(all(&mut e), source);
}
#[test]
fn framed_code_shares_source_colors_and_copy_preserves_tabs_and_blank_lines() {
    let body = "\tlet count = 42;\n\n    count + 1\n";
    let mut preview = editor(&format!("```rust\n{body}```\n"), "blocks.md", Mode::View);
    let screen = draw(&mut preview, 60);
    assert_eq!(screen[position(&screen, "Rust")].fg, Color::DarkGray);
    let p = position(&screen, "let count");
    assert_eq!(screen[(2, p.1)].symbol(), "│");
    assert_eq!(screen[(2, p.1)].fg, Color::DarkGray);
    let mut standalone = editor(body, "code.rs", Mode::Edit);
    let source = draw(&mut standalone, 56);
    for token in ["let", "42", "count +"] {
        assert_eq!(
            screen[position(&screen, token)].fg,
            source[position(&source, token)].fg
        );
    }
    assert_eq!(all(&mut preview), body.trim_end_matches('\n'));
    let selected = draw(&mut preview, 60);
    assert!(!selected[(2, p.1)].modifier.contains(Modifier::REVERSED));
    assert!(
        !selected[position(&selected, "Rust")]
            .modifier
            .contains(Modifier::REVERSED)
    );
}
#[test]
fn text_fences_wrap_like_standalone_text_and_markdown_keeps_section_rules() {
    let body = "  - [x] alpha beta gamma delta epsilon zeta eta theta";
    let mut preview = editor(&format!("```text\n{body}\n```"), "blocks.md", Mode::View);
    let wrapped = draw(&mut preview, 36);
    let mut standalone = editor(body, "words.txt", Mode::Edit);
    let source = draw(&mut standalone, 34);
    for y in 2..8 {
        let expected: String = (2..36).map(|x| source[(x, y)].symbol()).collect();
        if expected.trim().is_empty() {
            break;
        }
        let actual: String = (4..38).map(|x| wrapped[(x, y + 1)].symbol()).collect();
        assert_eq!(actual, expected);
    }
    assert_eq!(all(&mut preview), body);
    preview.act(Action::ToggleWrap);
    draw(&mut preview, 36);
    assert_eq!(all(&mut preview), body);
    let markdown = "# Section\n\n| A very long table header | Second header |\n| --- | --- |\n| First | Second |\n";
    let mut nested = editor(
        &format!("````markdown\n{markdown}````"),
        "blocks.md",
        Mode::View,
    );
    let view = draw(&mut nested, 36);
    assert_eq!(view[position(&view, "Section")].fg, Color::Magenta);
    assert!(
        view[position(&view, "Section")]
            .modifier
            .contains(Modifier::UNDERLINED)
    );
    assert_eq!(all(&mut nested), markdown.trim_end_matches('\n'));
}
#[test]
fn responsive_tables_keep_cells_links_and_clipboard_logical_rows() {
    let source = "| ID | Description | Status |\n| ---: | :--- | :---: |\n| 7 | **Important** words with a [long link label that wraps across rows](https://example.com) and 日本語 text. | Ready |\n| 12 | alpha  beta gamma delta epsilon zeta eta theta | Waiting |\n";
    let mut e = editor(source, "tables.md", Mode::View);
    e.wrap = false;
    draw(&mut e, 120);
    let expected = all(&mut e);
    e.wrap = true;
    for width in [50, 64, 110] {
        let screen = draw(&mut e, width);
        let border: String = (2..2 + width).map(|x| screen[(x, 2)].symbol()).collect();
        assert!(!border.contains(['┌', '┐', '└', '┘']), "{width}: {border}");
        assert_eq!(border.contains('│'), width >= 60, "{width}: {border}");
        assert_eq!(all(&mut e), expected, "width {width}");
        let p = position(&screen, "Important");
        assert!(screen[p].modifier.contains(Modifier::BOLD));
        let p = position(&screen, "long link");
        mouse(&mut e, MK::Down(B::Left), p.0, p.1);
        let outcome = e.handle(Event::Mouse(MouseEvent {
            kind: MK::Up(B::Left),
            column: p.0,
            row: p.1,
            modifiers: M::NONE,
        }));
        assert_eq!(
            outcome,
            Outcome::CopyRequested("https://example.com".into())
        );
    }
}
#[test]
fn many_columns_keep_readable_minima_and_visual_diagrams_remain_unframed() {
    let source = "| Description one | Description two | Description three | Description four |\n| --- | --- | --- | --- |\n| A very long description | Another very long description | Third long description | Fourth long description |\n";
    let mut e = editor(source, "wide.md", Mode::View);
    e.tables.columns = tapp_ui::renderers::table_layout::Separator::Always;
    let screen = draw(&mut e, 36);
    let top: String = (2..38).map(|x| screen[(x, 2)].symbol()).collect();
    assert!(!top.contains('┐'));
    assert!(top.find('│').unwrap() >= 14);
    assert!(all(&mut e).contains("A very long description\tAnother very long description"));
    let mut graph = editor(
        "```mermaid\nflowchart LR\n A[Start] --> B[Finish]\n```",
        "graph.md",
        Mode::View,
    );
    let screen = draw(&mut graph, 64);
    position(&screen, "Start");
    assert!(!all(&mut graph).contains("Plain Text"));
    let text: String = screen.content.iter().map(|cell| cell.symbol()).collect();
    assert!(!text.contains("-visual") && !text.contains("Mermaid"));
}

#[test]
fn nested_frames_keep_copy_positions_and_wrapped_cells_exclude_padding() {
    let mut code = editor(
        "> [!NOTE]\n> ```text\n>   - alpha beta gamma delta epsilon zeta eta theta\n> ```\n",
        "nested.md",
        Mode::View,
    );
    draw(&mut code, 36);
    let copied = all(&mut code);
    assert!(
        copied.contains("  - alpha beta gamma delta epsilon zeta eta theta"),
        "{copied:?}"
    );
    assert!(!copied.contains('│') && !copied.contains('┃'));

    let mut table = editor(
        "> | Key | Description |\n> | --- | --- |\n> | A | alpha beta gamma delta epsilon zeta eta theta followed by more words to demonstrate wrapping across visual rows omega |\n",
        "nested.md",
        Mode::View,
    );
    let screen = draw(&mut table, 96);
    assert_eq!(
        all(&mut table),
        "Key\tDescription\nA\talpha beta gamma delta epsilon zeta eta theta followed by more words to demonstrate wrapping across visual rows omega"
    );
    let first = position(&screen, "alpha");
    let last = position(&screen, "omega");
    mouse(&mut table, MK::Down(B::Left), first.0, first.1);
    mouse(&mut table, MK::Drag(B::Left), last.0 + 5, last.1);
    mouse(&mut table, MK::Up(B::Left), last.0 + 5, last.1);
    assert_eq!(
        table.act(Action::Copy),
        Outcome::CopyRequested("alpha beta gamma delta epsilon zeta eta theta followed by more words to demonstrate wrapping across visual rows omega".into())
    );
}

#[test]
fn copy_button_tracks_viewport_and_copies_complete_original_code() {
    use crossterm::event::{KeyCode, KeyEvent};
    let body = "\tlet long_name = \"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz\";  \n\n";
    let mut e = editor(&format!("```rust\n{body}```"), "copy.md", Mode::View);
    for (width, horizontal) in [(36, 0), (36, 12), (50, 0)] {
        e.horizontal = horizontal;
        let screen = draw(&mut e, width);
        let (x, y) = position(&screen, "⧉ Copy");
        assert_eq!(x, 2 + width - 6);
        mouse(&mut e, MK::Down(B::Left), x, y);
        assert_eq!(
            e.handle(Event::Mouse(MouseEvent {
                kind: MK::Up(B::Left),
                column: x,
                row: y,
                modifiers: M::NONE
            })),
            Outcome::CopyRequested(body.into())
        );
        assert_eq!(e.mode, Mode::View);
        assert!(!all(&mut e).contains("Copy"));
    }
    e.handle(Event::Key(KeyEvent::new(KeyCode::Tab, M::NONE)));
    assert_eq!(
        e.handle(Event::Key(KeyEvent::new(KeyCode::Enter, M::NONE))),
        Outcome::CopyRequested(body.into())
    );
}

#[test]
fn list_markers_and_compact_tasks_share_color_with_source_brackets_preserved() {
    let source = "- Bullet\n\n1. Ordered\n\n- [x] Checked\n- [ ] Pending\n\n> - [x] Quoted\n";
    for mode in [Mode::View, Mode::Edit] {
        let mut e = editor(source, "lists.md", mode);
        let screen = draw(&mut e, 60);
        let bullet = if mode == Mode::View {
            "○ Bullet"
        } else {
            "- Bullet"
        };
        assert_eq!(screen[position(&screen, bullet)].fg, Color::LightBlue);
        assert_eq!(screen[position(&screen, "1. Ordered")].fg, Color::LightBlue);
        if mode == Mode::View {
            for task in ["☑ Checked", "☐ Pending", "☑ Quoted"] {
                let (x, y) = position(&screen, task);
                assert_eq!(screen[(x, y)].fg, Color::LightBlue);
                assert_eq!(screen[(x + 1, y)].symbol(), " ");
                assert_ne!(screen[(x + 2, y)].fg, Color::LightBlue);
            }
            continue;
        }
        for task in ["[x] Checked", "[ ] Pending", "[x] Quoted"] {
            let (x, y) = position(&screen, task);
            assert_eq!(screen[(x, y)].fg, Color::DarkGray);
            assert_eq!(screen[(x + 2, y)].fg, Color::DarkGray);
            assert_ne!(screen[(x + 1, y)].fg, Color::DarkGray);
        }
    }
}

#[test]
fn horizontal_preview_scroll_keeps_fitting_content_and_hit_testing_fixed() {
    let code = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let source = format!(
        "# Fixed heading\n\n[Fixed link](https://example.com)\n\n```rust\n{code}\nshort\n```\n\nFixed paragraph.\n"
    );
    let mut e = editor(&source, "scroll.md", Mode::View);
    let before = draw(&mut e, 36);
    e.horizontal = 8;
    let after = draw(&mut e, 36);
    for label in [
        "Fixed heading",
        "Fixed link",
        "Rust",
        "short",
        "Fixed paragraph.",
        "⧉ Copy",
    ] {
        assert_eq!(position(&before, label), position(&after, label), "{label}");
    }
    let (x, y) = position(&after, "ghijkl");
    assert_eq!(x, 2);
    mouse(&mut e, MK::Down(B::Left), x, y);
    mouse(&mut e, MK::Drag(B::Left), x + 6, y);
    assert_eq!(e.act(Action::Copy), Outcome::CopyRequested("ghijkl".into()));
    let (x, y) = position(&after, "Fixed link");
    mouse(&mut e, MK::Down(B::Left), x, y);
    assert_eq!(
        e.handle(Event::Mouse(MouseEvent {
            kind: MK::Up(B::Left),
            column: x,
            row: y,
            modifiers: M::NONE
        })),
        Outcome::CopyRequested("https://example.com".into())
    );
    e.act(Action::SelectAll);
    let copied = e.act(Action::Copy);
    assert!(
        matches!(copied,Outcome::CopyRequested(s) if s.contains(code) && s.contains("Fixed paragraph."))
    );
}

#[test]
fn command_shortcut_replaces_preview_context_menu() {
    use crossterm::event::{KeyCode, KeyEvent};
    let mut e = editor("[A link](https://example.com)", "menu.md", Mode::View);
    let screen = draw(&mut e, 60);
    let (x, y) = position(&screen, "A link");
    mouse(&mut e, MK::Down(B::Right), x, y);
    position(&draw(&mut e, 60), "Copy URL");
    e.handle(Event::Key(KeyEvent::new(KeyCode::Char('k'), M::CONTROL)));
    let screen = draw(&mut e, 60);
    position(&screen, "Command center");
    let text: String = screen.content.iter().map(|c| c.symbol()).collect();
    assert!(!text.contains("Copy URL"));
    assert_eq!(e.prompt.as_ref().unwrap().kind, editio::PromptKind::Palette);
}

#[test]
fn csv_and_tsv_edit_columns_have_distinct_colors_without_changing_text() {
    for (path, separator) in [("columns.csv", ","), ("columns.tsv", "\t")] {
        let source = format!(
            "first{separator}second{separator}third{separator}fourth{separator}fifth\n1{separator}2{separator}3{separator}4{separator}5"
        );
        let mut e = editor(&source, path, Mode::Edit);
        let screen = draw(&mut e, 80);
        for (word, color) in [
            ("first", Color::Reset),
            ("second", Color::Cyan),
            ("third", Color::Yellow),
            ("fourth", Color::Magenta),
            ("fifth", Color::Green),
        ] {
            assert_eq!(screen[position(&screen, word)].fg, color);
        }
        assert_eq!(all(&mut e), source);
    }
}

#[test]
fn shift_click_extends_rendered_selection_without_activating_link() {
    let mut e = editor(
        "alpha [beta](https://example.com) gamma",
        "shift.md",
        Mode::View,
    );
    let s = draw(&mut e, 60);
    let (x, y) = position(&s, "alpha");
    mouse(&mut e, MK::Down(B::Left), x, y);
    let (x, y) = position(&s, "beta");
    let result = e.handle(Event::Mouse(MouseEvent {
        kind: MK::Down(B::Left),
        column: x,
        row: y,
        modifiers: M::SHIFT,
    }));
    assert_eq!(result, Outcome::Handled);
    assert_eq!(e.act(Action::Copy), Outcome::CopyRequested("alpha".into()));
}

#[test]
fn list_quotes_and_post_fence_text_keep_container_alignment() {
    let source = "- Item\n  > Nested quote\n- Code\n  ```ts\n  const value = 1\n  ```\n  Continuation\n\n  Second paragraph\n\n- Outer\n   - Inner\n     > Deep quote\n";
    for width in [28, 80] {
        let mut e = editor(source, "indent.md", Mode::View);
        let s = draw(&mut e, width);
        assert_eq!(position(&s, "Continuation").0, position(&s, "Code").0);
        assert_eq!(position(&s, "Second paragraph").0, position(&s, "Code").0);
        let (x, y) = position(&s, "Nested quote");
        assert_eq!(x, position(&s, "Item").0 + 2);
        assert_eq!(s[(x - 2, y)].symbol(), "│");
        assert!(position(&s, "Deep quote").0 > x);
    }
}
#[test]
fn extra_list_indent_keeps_bullet_color_in_source() {
    let mut e = editor(
        "- Parent\n   - Three\n- Parent\n    - Four\n",
        "indent.md",
        Mode::Edit,
    );
    let s = draw(&mut e, 80);
    let expected = s[(2, 2)].fg;
    assert_eq!(s[(5, 3)].fg, expected);
    assert_eq!(s[(6, 5)].fg, expected);
}
#[test]
fn normal_markdown_whitespace_and_setext_semantics_are_preserved() {
    let mut e = editor(
        "Paragraph\n   Continuation\n\nOr:\n    Still prose\n\n- Title\n  -\n",
        "indent.md",
        Mode::View,
    );
    let s = draw(&mut e, 80);
    assert_eq!(position(&s, "Paragraph Continuation").0, 2);
    assert_eq!(position(&s, "Or: Still prose").0, 2);
    let (x, y) = position(&s, "Title");
    assert!(s[(x, y)].modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn list_quote_wrapping_repeats_border_and_copy_excludes_container_padding() {
    let body = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let mut e = editor(&format!("- Parent\n  > {body}\n"), "nested.md", Mode::View);
    let screen = draw(&mut e, 24);
    let (x, y) = position(&screen, "alpha");
    assert_eq!(screen[(x - 2, y + 1)].symbol(), "│");
    let copied = all(&mut e);
    assert!(!copied.contains('│'));
    assert!(copied.contains(body), "{copied:?}");
}

#[test]
fn tab_nested_bullet_keeps_color_when_parser_range_starts_on_previous_newline() {
    let source = "# Agent team organization examples\n\nPersonal:\n\n- colored\n\t- not colored\n  - colored";
    for source in [source.to_owned(), source.replace('\n', "\r\n")] {
        for wrap in [false, true] {
            let mut e = editor(&source, "org.md", Mode::Edit);
            e.wrap = wrap;
            let s = draw(&mut e, 80);
            let expected = s[(2, 6)].fg;
            assert_eq!(s[(4, 7)].symbol(), "-");
            assert_eq!(s[(4, 7)].fg, expected);
            assert_eq!(s[(4, 8)].fg, expected);
        }
    }
}
