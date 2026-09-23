use editio::{
    Action, Editor, Mode,
    buffer::{Buffer, Cursor},
};
use ratatui::{Terminal, backend::TestBackend};
use std::time::{Duration, Instant};

fn editor(text: &str, path: &str) -> Editor {
    let mut b = Buffer::new(text);
    b.path = Some(path.into());
    Editor::new(b)
}
fn draw(e: &mut Editor, width: u16, height: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        e.poll_background();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        if !e.has_background_work() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    (0..height - 1)
        .map(|y| {
            (0..width)
                .map(|x| t.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn cycles(e: &mut Editor, width: u16, height: u16) {
    let original = draw(e, width, height);
    let initial = (e.mode, e.scroll, e.horizontal, e.buffer.cursors.clone());
    e.act(Action::ToggleMode);
    let other = draw(e, width, height);
    let other_position = (e.scroll, e.horizontal, e.buffer.cursors.clone());
    for _ in 0..40 {
        e.act(Action::ToggleMode);
        assert_eq!(draw(e, width, height), original);
        assert_eq!(
            (e.mode, e.scroll, e.horizontal, e.buffer.cursors.clone()),
            initial
        );
        e.act(Action::ToggleMode);
        assert_eq!(draw(e, width, height), other);
        assert_eq!(
            (e.scroll, e.horizontal, e.buffer.cursors.clone()),
            other_position
        );
    }
    e.act(Action::ToggleMode);
    draw(e, width, height);
}

#[test]
fn plain_wrapped_and_horizontal_views_keep_exact_positions_without_drift() {
    let text = (0..150)
        .map(|i| format!("row{i:03} {} 猫🙂\n", "long text ".repeat(15)))
        .collect::<String>();
    for path in ["file.txt", "file.rs", "file.md"] {
        for wrap in [false, true] {
            let mut e = editor(&text, path);
            e.source = true;
            e.wrap = wrap;
            draw(&mut e, 48, 15);
            e.scroll = 55;
            e.horizontal = if wrap { 0 } else { 20 };
            let before = e.scroll;
            e.act(Action::ToggleMode);
            draw(&mut e, 48, 15);
            assert_eq!(e.scroll, before, "{path} wrap={wrap}");
            cycles(&mut e, 48, 15);
        }
    }
}

#[test]
fn markdown_and_tables_keep_corresponding_text_and_round_trip_exactly() {
    for (path, text) in [
        ("file.md", (0..60).map(|i| format!("<!-- hidden {i} -->\n\n## Heading{i:03}\n\n- Unique{i:03} **bold** and [link](https://example.com/{i})\n\n```rust\nlet value{i:03} = {i};\n```\n\n")).collect::<String>()),
        ("file.csv", "ID,Description\n".to_owned() + &(0..100).map(|i| format!("item{i:03},\"unique{i:03} multiline\nsecond line\"\n")).collect::<String>()),
        ("file.tsv", "ID\tDescription\n".to_owned() + &(0..100).map(|i| format!("item{i:03}\tunique{i:03} value\n")).collect::<String>()),
    ] {
        let mut e = editor(&text, path);
        draw(&mut e, 50, 16);
        e.scroll = 65;
        let visible = draw(&mut e, 50, 16);
        let marker = visible.split_whitespace().find(|s| s.starts_with("Heading") || s.starts_with("item")).unwrap().to_owned();
        let before = e.scroll;
        e.act(Action::ToggleMode);
        let edit = draw(&mut e, 50, 16);
        assert!(edit.contains(&marker), "{path}: missing {marker}\n{edit}");
        assert!(e.scroll > 0);
        e.act(Action::ToggleMode);
        draw(&mut e, 50, 16);
        assert_eq!(e.scroll, before);
        cycles(&mut e, 50, 16);
        draw(&mut e, 28, 10);
        cycles(&mut e, 28, 10);
        e.scroll = usize::MAX;
        draw(&mut e, 28, 10);
        cycles(&mut e, 28, 10);
    }
}

#[test]
fn edit_cursor_keeps_screen_row_and_new_navigation_replaces_old_anchor() {
    let text = (0..120)
        .map(|i| format!("# Heading{i:03}\n\nParagraph{i:03}\n\n"))
        .collect::<String>();
    let mut e = editor(&text, "file.md");
    e.mode = Mode::Edit;
    draw(&mut e, 60, 18);
    e.buffer.cursors = vec![Cursor::at(e.buffer.text.line_to_char(80))];
    e.scroll = 75;
    let original = draw(&mut e, 60, 18);
    assert!(original.lines().nth(5).unwrap().contains("Heading020"));
    e.act(Action::ToggleMode);
    let view = draw(&mut e, 60, 18);
    assert!(
        view.lines().nth(5).unwrap().contains("Heading020"),
        "{view}"
    );
    cycles(&mut e, 60, 18);
    e.scroll += 35;
    let view = draw(&mut e, 60, 18);
    e.act(Action::ToggleMode);
    draw(&mut e, 60, 18);
    assert!(e.scroll > 75);
    e.act(Action::ToggleMode);
    assert_eq!(draw(&mut e, 60, 18), view);
    cycles(&mut e, 60, 18);
}

#[test]
fn edits_and_fast_toggles_while_preview_is_pending_do_not_restore_stale_positions() {
    let mut e = editor(&"# Heading\n\nparagraph\n\n".repeat(120), "file.md");
    e.mode = Mode::Edit;
    draw(&mut e, 60, 14);
    e.scroll = 100;
    e.buffer.cursors = vec![Cursor::at(e.buffer.text.line_to_char(104))];
    let before = draw(&mut e, 60, 14);
    e.act(Action::ToggleMode);
    e.act(Action::ToggleMode);
    assert_eq!(e.mode, Mode::Edit);
    assert_eq!(draw(&mut e, 60, 14), before);
    e.buffer.insert("new text\n\n");
    draw(&mut e, 60, 14);
    cycles(&mut e, 60, 14);
}

#[test]
fn a_deeply_wrapped_markdown_paragraph_keeps_the_visible_fragment() {
    let text = (0..400)
        .map(|i| format!("token{i:03} "))
        .collect::<String>();
    let mut e = editor(&text, "paragraph.md");
    draw(&mut e, 40, 12);
    e.scroll = 55;
    let view = draw(&mut e, 40, 12);
    let token = view.split_whitespace().next().unwrap();
    e.act(Action::ToggleMode);
    let edit = draw(&mut e, 40, 12);
    assert!(
        edit.lines().next().unwrap().contains(token),
        "wanted {token}\n{edit}"
    );
    cycles(&mut e, 40, 12);
    // Move to another fragment in edit mode; a new view anchor must follow it.
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    e.handle(Event::Key(KeyEvent::new(
        KeyCode::PageDown,
        KeyModifiers::NONE,
    )));
    let edit = draw(&mut e, 40, 12);
    let caret = e.buffer.cursors[0].head;
    let next_token = e
        .buffer
        .text
        .slice(caret..e.buffer.text.len_chars())
        .to_string()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    e.act(Action::ToggleMode);
    let view = draw(&mut e, 40, 12);
    assert!(
        view.contains(&next_token),
        "wanted {next_token}\n{view}\nfrom\n{edit}"
    );
    cycles(&mut e, 40, 12);
}

#[test]
fn rendered_diagrams_details_and_tables_do_not_accumulate_rounding_drift() {
    for file in [
        "13-flowchart-layouts.md",
        "21-details-and-comments.md",
        "26-responsive-tables.md",
    ] {
        let text = std::fs::read_to_string(format!("examples/markdown/{file}")).unwrap();
        let mut e = editor(&text, file);
        e.line_numbers = true;
        draw(&mut e, 48, 14);
        e.scroll = 17;
        draw(&mut e, 48, 14);
        cycles(&mut e, 48, 14);
    }
}

#[test]
fn markdown_sticky_headers_keep_viewport_round_trips_stable_near_table_end() {
    let table = (0..60)
        .map(|i| format!("| item{i:03} | unique{i:03} description |\n"))
        .collect::<String>();
    let text = format!(
        "Intro\n\n| Item | Description |\n| --- | --- |\n{table}\n{}",
        "Following paragraph.\n\n".repeat(30)
    );
    let mut e = editor(&text, "sticky.md");
    for width in [38, 80] {
        draw(&mut e, width, 14);
        for scroll in [20, 60, 65] {
            e.scroll = scroll;
            draw(&mut e, width, 14);
            cycles(&mut e, width, 14);
        }
    }
    assert_eq!(e.buffer.text.to_string(), text);
}

#[test]
fn json_pretty_view_toggle_and_mode_round_trips_preserve_source() {
    let source = format!(
        "{{\"records\":[{}]}}",
        (0..80)
            .map(|i| format!("{{\"id\":{i},\"name\":\"record{i:03}\"}}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut e = editor(&source, "records.JSON");
    let formatted = draw(&mut e, 80, 20);
    assert!(e.rendered_view());
    assert!(
        e.matching_commands()
            .iter()
            .any(|c| matches!(c.action, Action::ToggleJsonFormatting))
    );
    assert!(formatted.contains("  \"records\": ["));
    assert_eq!(e.buffer.text.to_string(), source);
    e.scroll = 30;
    cycles(&mut e, 80, 20);
    e.act(Action::ToggleJsonFormatting);
    assert!(!e.rendered_view());
    let raw = draw(&mut e, 80, 20);
    assert!(raw.contains("{\"records\":["));
    e.act(Action::ToggleMode);
    draw(&mut e, 80, 20);
    assert!(
        !e.matching_commands()
            .iter()
            .any(|c| matches!(c.action, Action::ToggleJsonFormatting))
    );
    e.act(Action::ToggleMode);
    assert!(!e.rendered_view());
    e.act(Action::ToggleJsonFormatting);
    assert_eq!(draw(&mut e, 80, 20), formatted);
    assert_eq!(e.buffer.text.to_string(), source);
}

#[test]
fn unsaved_documents_detect_preview_format_and_keep_the_buffer_untitled() {
    for (source, expected, format) in [
        (
            r#"{"name":"unsaved","items":[1,2]}"#,
            "  \"name\": \"unsaved\",",
            "json",
        ),
        (
            "# Unsaved title\n\n**Formatted** text",
            "Formatted text",
            "markdown",
        ),
        ("Name,Value\nAlice,42\nBob,17", "Alice", "csv"),
        ("Name\tValue\nAlice\t42\nBob\t17", "Alice", "tsv"),
    ] {
        let mut e = Editor::new(Buffer::new(""));
        e.mode = Mode::Edit;
        e.paste(source);
        draw(&mut e, 80, 16);
        let dirty = e.buffer.dirty();
        e.act(Action::ToggleMode);
        let view = draw(&mut e, 80, 16);
        assert!(e.rendered_view());
        assert!(view.contains(expected), "{view}");
        assert_eq!(e.preference_type(), format);
        if matches!(format, "csv" | "tsv") {
            assert!(view.contains('│'));
        }
        assert!(e.buffer.path.is_none());
        assert_eq!(e.buffer.text.to_string(), source);
        assert_eq!(e.buffer.dirty(), dirty);
        cycles(&mut e, 80, 16);
        e.act(Action::ToggleMode);
        e.act(Action::SelectAll);
        e.paste("plain text now");
        e.act(Action::ToggleMode);
        draw(&mut e, 80, 16);
        assert!(!e.rendered_view());
        assert!(!e.is_json() && !e.is_markdown());
    }
    let mut named = editor(r#"{"name":"literal"}"#, "literal.txt");
    draw(&mut named, 80, 16);
    assert!(!named.rendered_view());
}

#[test]
fn editing_actions_do_not_leave_saved_untitled_documents_in_source_view() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use editio::Outcome;
    let dir = tempfile::tempdir_in(".").unwrap();
    for (index, action) in [
        Action::SelectAll,
        Action::CursorDown,
        Action::AddNext,
        Action::AllMatches,
        Action::MatchBracket,
    ]
    .into_iter()
    .enumerate()
    {
        let path =
            std::path::Path::new(dir.path().file_name().unwrap()).join(format!("new-{index}.md"));
        assert!(path.is_relative());
        let mut e = Editor::new(Buffer::new(""));
        e.mode = Mode::Edit;
        e.paste(
            "# Convenor concepts\n\nUsers only see agents (agents).\n\nAgents have:\n- tasks\n",
        );
        draw(&mut e, 80, 16);
        let offset = e
            .buffer
            .text
            .to_string()
            .find(if matches!(action, Action::MatchBracket) {
                "("
            } else {
                "agents"
            })
            .unwrap();
        e.buffer.cursors = vec![Cursor::at(offset)];
        e.act(action);
        e.act(Action::Save);
        e.handle(Event::Paste(path.to_string_lossy().into_owned()));
        assert_eq!(
            e.handle(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE
            ))),
            Outcome::SaveRequested
        );
        let saved = e.buffer.save();
        e.save_finished(saved);
        let source = e.buffer.text.to_string();
        e.act(Action::ToggleMode);
        let view = draw(&mut e, 80, 16);
        assert!(
            e.rendered_view(),
            "{action:?}: saved Markdown must render in View\n{view}"
        );
        assert!(!view.contains("# Convenor concepts"), "{view}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        cycles(&mut e, 80, 16);
        e.act(Action::ToggleMode);
        e.buffer.insert("edited ");
        e.act(Action::ToggleMode);
        draw(&mut e, 80, 16);
        assert!(e.rendered_view());
    }
}

#[test]
fn explicit_source_preference_survives_editing_and_mode_switches() {
    let mut e = editor("# Title\n\n**body**\n", "explicit.md");
    draw(&mut e, 80, 16);
    e.act(Action::ToggleSource);
    e.act(Action::ToggleMode);
    draw(&mut e, 80, 16);
    e.act(Action::SelectAll);
    e.paste("# Replacement\n\n**body**\n");
    e.act(Action::ToggleMode);
    let view = draw(&mut e, 80, 16);
    assert!(!e.rendered_view());
    assert!(view.contains("# Replacement"));
    e.act(Action::ToggleSource);
    let view = draw(&mut e, 80, 16);
    assert!(e.rendered_view());
    assert!(!view.contains("# Replacement"));
}

#[test]
fn edit_line_end_anchor_survives_nonuniform_large_markdown_and_new_edits() {
    let source = format!(
        "{}{}",
        "<!-- hidden -->\n".repeat(3500),
        (0..300)
            .map(|i| format!("Paragraph{i:03} content.\n\n"))
            .collect::<String>()
    );
    let mut e = editor(&source, "large.md");
    // Retain an old preview while editing, then toggle before revision invalidation.
    draw(&mut e, 70, 20);
    e.mode = Mode::Edit;
    draw(&mut e, 70, 20);
    let line = 3500 + 200 * 2;
    e.buffer.cursors = vec![Cursor::at(
        e.buffer.text.line_to_char(line) + "Paragraph200 content.".len(),
    )];
    e.scroll = line - 8;
    draw(&mut e, 70, 20);
    e.buffer.insert(" updated");
    // Toggle before a draw has invalidated the old presentation revision.
    e.act(Action::ToggleMode);
    let view = draw(&mut e, 70, 20);
    assert!(view.contains("Paragraph200 content. updated"), "{view}");
    let row = view
        .lines()
        .position(|l| l.contains("Paragraph200"))
        .unwrap();
    assert!(row.abs_diff(8) <= 1, "anchor row {row}: {view}");
    cycles(&mut e, 70, 20);
}

#[test]
fn manual_scroll_keeps_top_text_even_when_caret_is_still_visible() {
    use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
    let source = (0..180)
        .map(|i| format!("## Heading{i:03}\n\nParagraph{i:03}\n\n"))
        .collect::<String>();
    let mut e = editor(&source, "scroll.md");
    e.mode = Mode::Edit;
    draw(&mut e, 60, 24);
    e.buffer.cursors = vec![Cursor::at(e.buffer.text.line_to_char(92))];
    e.scroll = 80;
    draw(&mut e, 60, 24);
    e.handle(Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    }));
    let edit = draw(&mut e, 60, 24);
    let first = edit.lines().find(|l| !l.trim().is_empty()).unwrap();
    let token = first
        .split_whitespace()
        .find(|s| s.starts_with("Heading") || s.starts_with("Paragraph"))
        .unwrap();
    e.act(Action::ToggleMode);
    let view = draw(&mut e, 60, 24);
    assert!(
        view.lines().take(3).any(|line| line.contains(token)),
        "wanted {token}: {view}"
    );
    cycles(&mut e, 60, 24);
}

#[test]
fn cursor_context_crossing_rendered_wraps_preserves_top_and_bottom_edges() {
    let source = "<!-- omitted -->\n".repeat(700) + &(0..180).map(|i| format!("Paragraph{i:03} starts here, with enough ordinary words to span several rendered rows and exercise a caret within a line rather than at its endpoints. Each section contains distinct text {i} for reliable navigation.\n\n")).collect::<String>();
    for width in [60, 92, 120] {
        for screen_row in [0, 1, 16] {
            let mut e = editor(&source, "wrapped-anchor.md");
            e.mode = Mode::Edit;
            e.wrap = true;
            draw(&mut e, width, 20);
            let caret = e.buffer.text.line_to_char(700 + 120 * 2) + 19;
            let layout =
                tapp_ui::renderers::wrap::Layout::new(&e.buffer.text, 0, width as usize, |_, _| {
                    false
                });
            e.buffer.cursors = vec![Cursor::at(caret)];
            e.scroll = layout.row_at(caret).saturating_sub(screen_row);
            let edit = draw(&mut e, width, 20);
            let before = edit
                .lines()
                .position(|l| l.contains("Paragraph120"))
                .unwrap();
            e.act(Action::ToggleMode);
            let view = draw(&mut e, width, 20);
            let after = view
                .lines()
                .position(|l| l.contains("Paragraph120"))
                .unwrap_or_else(|| panic!("width {width}, row {screen_row}: {view}"));
            assert!(
                before.abs_diff(after) <= 1,
                "width {width}: {before} -> {after}"
            );
            cycles(&mut e, width, 20);
        }
    }
}
