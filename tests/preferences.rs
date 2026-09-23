use editio::{Action, Editor, Mode, Outcome, buffer::Buffer};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer as Screen};
fn editor(text: &str, path: &str) -> Editor {
    let mut b = Buffer::new(text);
    b.path = Some(path.into());
    Editor::new(b)
}
fn draw(e: &mut Editor, width: u16) -> Screen {
    let mut t = Terminal::new(TestBackend::new(width, 80)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while e.has_background_work() {
        assert!(std::time::Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(std::time::Duration::from_millis(2));
        t.draw(|f| e.draw(f, f.area())).unwrap();
    }
    t.backend().buffer().clone()
}
fn row(s: &Screen, y: u16) -> String {
    (0..s.area.width).map(|x| s[(x, y)].symbol()).collect()
}
#[test]
fn layout_preferences_round_trip_by_type_and_follow_save_as() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config/preferences.tsv");
    let mut first = editor("content", "one.md");
    first.enable_preferences(&path).unwrap();
    assert!(!path.exists());
    first.act(Action::ToggleWrap);
    first.act(Action::ToggleLineNumbers);
    first.act(Action::ToggleWidthLimit);
    let mut other = editor("different", "two.MARKDOWN");
    other.enable_preferences(&path).unwrap();
    assert!(!other.wrap && other.line_numbers && other.limit_width);
    let mut rust = editor("fn main() {}", "code.rs");
    rust.enable_preferences(&path).unwrap();
    assert!(rust.wrap && !rust.line_numbers && !rust.limit_width);
    rust.act(Action::ToggleLineNumbers);
    let mut upper = editor("", "another.RS");
    upper.enable_preferences(&path).unwrap();
    assert!(upper.line_numbers);
    other.buffer.path = Some("saved.txt".into());
    draw(&mut other, 80);
    assert!(other.wrap && !other.line_numbers && !other.limit_width);
    other.buffer.path = Some("back.md".into());
    draw(&mut other, 80);
    assert!(!other.wrap && other.line_numbers && other.limit_width);
    assert!(std::fs::read_to_string(path).unwrap().contains("rust\t"));
}
#[test]
fn preferences_merge_concurrent_types_and_do_not_overwrite_invalid_data() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("prefs.tsv");
    std::thread::scope(|scope| {
        for ext in ["md", "rs", "txt", "csv"] {
            let path = &path;
            scope.spawn(move || {
                let mut e = editor("", &format!("file.{ext}"));
                e.enable_preferences(path).unwrap();
                e.act(Action::ToggleLineNumbers);
                assert!(!e.message.contains("not saved"));
            });
        }
    });
    for ext in ["md", "rs", "txt", "csv"] {
        let mut e = editor("", &format!("file.{ext}"));
        e.enable_preferences(&path).unwrap();
        assert!(e.line_numbers);
    }
    let mut e = editor("", "file.md");
    e.enable_preferences(&path).unwrap();
    std::fs::write(&path, "invalid settings\n").unwrap();
    e.act(Action::ToggleLineNumbers);
    assert!(e.message.contains("not saved"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "invalid settings\n"
    );
    assert!(editor("", "other.md").enable_preferences(&path).is_err());
}
#[test]
fn width_limit_forces_text_wrap_and_preserves_copy_in_both_modes() {
    let text = "x".repeat(300);
    for name in ["file.txt", "file.rs", "file.md"] {
        for mode in [Mode::View, Mode::Edit] {
            let mut e = editor(&text, name);
            e.mode = mode;
            e.wrap = false;
            let before = draw(&mut e, 200);
            assert_eq!(row(&before, 0).trim().len(), 200);
            e.act(Action::ToggleWidthLimit);
            let capped = draw(&mut e, 200);
            assert_eq!(row(&capped, 0).trim().len(), 120);
            assert_eq!(row(&capped, 1).trim().len(), 120);
            e.act(Action::SelectAll);
            assert_eq!(e.act(Action::Copy), Outcome::CopyRequested(text.clone()));
            let small = draw(&mut e, 70);
            assert_eq!(row(&small, 0).trim().len(), 69);
            e.act(Action::ToggleWidthLimit);
            e.buffer.cursors = vec![editio::buffer::Cursor::at(0)];
            assert_eq!(row(&draw(&mut e, 200), 0).trim().len(), 200);
        }
    }
}
#[test]
fn width_limit_wraps_fences_and_table_cells_but_not_graphics() {
    let text = "a".repeat(300);
    let mut code = editor(&format!("```rust\n{text}\n```"), "file.md");
    code.wrap = false;
    code.limit_width = true;
    let screen = draw(&mut code, 200);
    assert!(row(&screen, 1).trim_end().chars().count() <= 120);
    assert!(row(&screen, 2).contains('a'));
    code.mode = Mode::Edit;
    code.source = true;
    let screen = draw(&mut code, 200);
    assert_eq!(row(&screen, 1).trim().len(), 120);
    for (name, source) in [
        (
            "table.md",
            format!(
                "| {} | B |\n| --- | --- |\n| text | other |",
                "header".repeat(30)
            ),
        ),
        ("table.csv", format!("A,B\n{},other", text)),
        (
            "diagram.md",
            "```mermaid\nflowchart LR\n A[Beginning] --> B[Middle] --> C[End]\n```".into(),
        ),
    ] {
        let mut e = editor(&source, name);
        let before = draw(&mut e, 200);
        e.act(Action::ToggleWidthLimit);
        let after = draw(&mut e, 200);
        if name == "diagram.md" {
            for y in 0..70 {
                assert_eq!(row(&before, y), row(&after, y), "{name} row {y}");
            }
        } else {
            assert_ne!(before, after, "table cells use the new 120-column cap");
            assert_eq!(e.buffer.text.to_string(), source);
        }
    }
}
#[test]
fn info_is_file_focused_and_palette_selection_has_no_chevron() {
    let mut e = editor("café\nhello", "notes.md");
    e.act(Action::Info);
    let s = draw(&mut e, 100);
    let text = (0..79).map(|y| row(&s, y)).collect::<String>();
    assert!(
        text.contains("Full file path")
            && text.contains("11 UTF-8 bytes")
            && text.contains("10 characters")
    );
    assert!(!text.contains("Ctrl+C / X / V"));
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    e.handle(Event::Key(KeyEvent::new(
        KeyCode::Char('k'),
        KeyModifiers::CONTROL,
    )));
    let s = draw(&mut e, 100);
    assert!(s.content.iter().all(|c| c.symbol() != "›"));
    let selected = s
        .content
        .iter()
        .find(|c| {
            c.bg == tapp_ui::theme::terminal_selection_background()
                && c.modifier.contains(ratatui::style::Modifier::BOLD)
        })
        .unwrap();
    assert!(
        !selected
            .modifier
            .contains(ratatui::style::Modifier::REVERSED)
    );
}

#[test]
fn json_preferences_keep_theme_and_file_types_independent() {
    use tapp_ui::theme::Theme;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("editio/config.json");
    let mut md = editor("", "file.md");
    md.enable_preferences(&path).unwrap();
    assert!(!path.exists());
    let mut rs = editor("", "file.rs");
    rs.enable_preferences(&path).unwrap();
    md.act(Action::ToggleTheme);
    rs.act(Action::ToggleLineNumbers);
    md.act(Action::ToggleWrap);
    let mut reopened = editor("", "next.rs");
    reopened.enable_preferences(&path).unwrap();
    assert_eq!(reopened.theme, Theme::TokyoNightOmarchy);
    assert!(reopened.line_numbers);
    reopened.buffer.path = Some("next.md".into());
    draw(&mut reopened, 80);
    assert!(!reopened.wrap && !reopened.line_numbers);
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.contains("\"file_types\"") && json.contains("\"line_numbers\""));
    for invalid in [
        "",
        "{broken",
        "{\"version\":2}",
        "{\"theme\":\"unknown\"}",
        "{\"extra\":true}",
    ] {
        std::fs::write(&path, invalid).unwrap();
        reopened.act(Action::ToggleTheme);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
        assert!(reopened.message.contains("Theme preference"));
        assert!(editor("", "file.md").enable_preferences(&path).is_err());
    }
}

#[test]
fn table_preferences_are_global_and_do_not_modify_other_settings() {
    use tapp_ui::renderers::table_layout::Separator;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config.json");
    let mut md = editor("", "file.md");
    md.enable_preferences(&path).unwrap();
    md.act(Action::CycleTableRows);
    md.act(Action::CycleTableColumns);
    md.act(Action::CycleTableColumns);
    let mut csv = editor("A,B\n1,2", "file.csv");
    csv.enable_preferences(&path).unwrap();
    assert_eq!(csv.tables.rows, Separator::Always);
    assert_eq!(csv.tables.columns, Separator::Never);
    csv.act(Action::ToggleLineNumbers);
    csv.act(Action::ToggleTheme);
    let mut rs = editor("", "file.rs");
    rs.enable_preferences(&path).unwrap();
    assert_eq!(rs.tables, csv.tables);
    assert!(!rs.line_numbers);
    md.act(Action::CycleTableRows);
    md.act(Action::CycleTableRows);
    assert_eq!(md.tables.rows, Separator::Adaptive);
}

#[test]
fn editor_updates_preserve_app_navigation_fields() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("config.json");
    std::fs::write(
        &path,
        r#"{"version":1,"theme":"terminal","navigation":{"tags":{"work":["/tmp/notes.md"]}}}"#,
    )
    .unwrap();
    let mut e = editio::Editor::new(editio::buffer::Buffer::new(""));
    e.enable_preferences(&path).unwrap();
    e.act(editio::Action::ToggleWidthLimit);
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(v["navigation"]["tags"]["work"][0], "/tmp/notes.md");
}
