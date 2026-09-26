use crossterm::event::{
    Event, KeyCode as K, KeyEvent, KeyModifiers as M, MouseButton, MouseEvent, MouseEventKind,
};
use editio::{Action, Editor, Mode, Outcome, PromptKind, buffer::Buffer};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer as Screen, style::Color};
fn key(e: &mut Editor, k: K) -> Outcome {
    e.handle(Event::Key(KeyEvent::new(k, M::NONE)))
}
#[test]
fn abort_is_explicit_cancel_safe_and_preserves_previous_saves() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("draft.txt");
    std::fs::write(&path, "saved").unwrap();
    for mode in [Mode::View, Mode::Edit] {
        let mut e = Editor::new(Buffer::open(&path).unwrap());
        e.mode = mode;
        assert!(
            e.command_contributions(1)
                .iter()
                .any(|c| c.label == "Abort editing" && c.available)
        );
        assert_eq!(e.act(Action::Abort), Outcome::Handled);
        assert!(screen(&draw(&mut e, 80, 24)).contains("Abort editing"));
        assert_eq!(key(&mut e, K::Enter), Outcome::Handled, "Cancel is default");
        e.buffer.insert("new ");
        e.buffer.save().unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        e.buffer.insert("unsaved ");
        e.act(Action::Abort);
        key(&mut e, K::Esc);
        assert!(e.buffer.dirty());
        e.act(Action::Abort);
        assert_eq!(key(&mut e, K::Char('a')), Outcome::AbortRequested);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
    }
}
fn draw(e: &mut Editor, w: u16, h: u16) -> Screen {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    t.backend().buffer().clone()
}
fn row(s: &Screen, y: u16) -> String {
    (0..s.area.width).map(|x| s[(x, y)].symbol()).collect()
}
fn screen(s: &Screen) -> String {
    (0..s.area.height)
        .map(|y| row(s, y))
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn nonexistent_paths_require_confirmation_before_creating_directories_and_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/nested/document.txt");
    let mut e = Editor::new(Buffer::open(&path).unwrap());
    e.mode = Mode::Edit;
    e.buffer.insert("saved text");
    assert_eq!(e.act(Action::Save), Outcome::Handled);
    assert!(screen(&draw(&mut e, 80, 24)).contains("Save and create this file?"));
    assert!(!dir.path().join("new").exists());
    key(&mut e, K::Esc);
    assert!(!path.exists());
    e.act(Action::Save);
    assert_eq!(key(&mut e, K::Char('s')), Outcome::SaveRequested);
    let result = e.buffer.save();
    assert_eq!(e.save_finished(result), Outcome::Handled);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "saved text");
    assert_eq!(e.act(Action::Save), Outcome::SaveRequested);
}
#[test]
fn quit_dialog_is_modal_and_untitled_save_and_quit_finishes_only_after_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a/b/note.txt");
    let mut e = Editor::new(Buffer::new(""));
    e.mode = Mode::Edit;
    e.buffer.insert("keep me");
    assert_eq!(e.act(Action::Quit), Outcome::Handled);
    let s = draw(&mut e, 80, 24);
    assert!(screen(&s).contains("Unsaved changes"));
    assert!(!row(&s, 23).contains("save & quit"));
    e.handle(Event::Paste("must not insert".into()));
    assert_eq!(e.buffer.text.to_string(), "keep me");
    assert_eq!(key(&mut e, K::Char('s')), Outcome::Handled);
    assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::SaveAs);
    e.handle(Event::Paste(format!("\"{}\"", path.display())));
    assert_eq!(
        key(&mut e, K::Enter),
        Outcome::SaveAsRequested(path.clone())
    );
    let result = e.buffer.save_as(&path);
    assert_eq!(e.save_finished(result), Outcome::QuitRequested);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "keep me");
}
#[test]
fn cancel_discard_failures_and_existing_file_conflicts_preserve_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.txt");
    let mut e = Editor::new(Buffer::open(&path).unwrap());
    e.buffer.insert("unsaved");
    e.act(Action::Quit);
    key(&mut e, K::Left);
    assert_eq!(key(&mut e, K::Enter), Outcome::Handled);
    assert!(e.buffer.dirty());
    e.act(Action::Quit);
    assert_eq!(key(&mut e, K::Char('s')), Outcome::SaveRequested);
    assert_eq!(
        e.save_finished(Err(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied
        ))),
        Outcome::Handled
    );
    assert_eq!(e.buffer.text.to_string(), "unsaved");
    assert!(e.buffer.dirty());
    e.act(Action::Save);
    key(&mut e, K::Char('s'));
    std::fs::write(&path, "external").unwrap();
    assert!(e.buffer.save().is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "external");
    e.act(Action::Quit);
    assert_eq!(key(&mut e, K::Char('d')), Outcome::QuitRequested);
}
#[test]
fn path_field_fits_content_wraps_colors_separators_and_supports_navigation() {
    let mut e = Editor::new(Buffer::new(""));
    e.act(Action::Save);
    let s = draw(&mut e, 80, 24);
    let top = (0..24).find(|&y| row(&s, y).contains('┌')).unwrap();
    let bottom = (0..24).find(|&y| row(&s, y).contains('└')).unwrap();
    assert_eq!(bottom - top, 2);
    e.handle(Event::Paste(
        "folder/subfolder/日本語/my document.txt".into(),
    ));
    let s = draw(&mut e, 28, 12);
    let top = (0..12).find(|&y| row(&s, y).contains('┌')).unwrap();
    let bottom = (0..12).find(|&y| row(&s, y).contains('└')).unwrap();
    assert!(bottom - top > 2 && bottom - top < 8);
    for y in top + 1..bottom {
        for x in 0..28 {
            if s[(x, y)].symbol() == "/" {
                assert_eq!(s[(x, y)].fg, Color::DarkGray);
            }
        }
    }
    e.handle(Event::Key(KeyEvent::new(K::Left, M::CONTROL)));
    assert!(
        e.prompt.as_ref().unwrap().value[e.prompt.as_ref().unwrap().cursor..]
            .starts_with("my document")
    );
    let before = e.prompt.as_ref().unwrap().cursor;
    key(&mut e, K::Up);
    assert!(e.prompt.as_ref().unwrap().cursor < before);
    let s = draw(&mut e, 28, 12);
    let y = (0..12).find(|&y| row(&s, y).contains("folder/")).unwrap();
    let line = row(&s, y);
    let x = line[..line.find("folder/").unwrap()].chars().count() as u16;
    e.handle(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: M::NONE,
    }));
    assert_eq!(e.prompt.as_ref().unwrap().cursor, 0);
    key(&mut e, K::Esc);
    assert!(e.prompt.is_none());
}
#[test]
fn filename_errors_allow_retry_and_tiny_confirmation_can_cancel() {
    let mut e = Editor::new(Buffer::new(""));
    e.act(Action::Save);
    key(&mut e, K::Enter);
    assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::SaveAs);
    assert!(e.message.contains("Filename cannot be empty"));
    key(&mut e, K::Esc);
    e.buffer.insert("modified");
    e.act(Action::Quit);
    e.message.clear();
    let s = draw(&mut e, 16, 4);
    assert!(screen(&s).contains("Save [s]"));
    key(&mut e, K::Left);
    let s = draw(&mut e, 16, 4);
    assert!(screen(&s).contains("Cancel"));
    assert_eq!(key(&mut e, K::Enter), Outcome::Handled);
}

#[test]
fn buttons_follow_risk_colors_visual_order_and_default_activation() {
    use ratatui::style::Modifier;
    for create in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut e = Editor::new(Buffer::open(&dir.path().join("new.txt")).unwrap());
        e.buffer.insert("content");
        e.act(if create { Action::Save } else { Action::Quit });
        let s = draw(&mut e, 80, 24);
        let primary = if create { "Create [s]" } else { "Save [s]" };
        let y = (0..24).find(|&y| row(&s, y).contains(primary)).unwrap();
        let line = row(&s, y);
        let col = |needle: &str| line[..line.find(needle).unwrap()].chars().count() as u16;
        let primary_x = col(primary);
        let cancel_x = col("Cancel");
        assert!(cancel_x < primary_x);
        assert!(line.trim_end().ends_with(&format!("{primary}  │")));
        assert_eq!(s[(primary_x, y)].fg, Color::Cyan);
        assert!(s[(primary_x, y)].modifier.contains(Modifier::REVERSED));
        assert_eq!(s[(cancel_x, y)].fg, Color::Reset);
        if !create {
            let discard_x = col("Discard");
            assert!(discard_x < cancel_x);
            assert_eq!(s[(discard_x, y)].fg, Color::Red);
        }
        assert_eq!(key(&mut e, K::Enter), Outcome::SaveRequested);
    }
    let mut e = Editor::new(Buffer::new(""));
    e.buffer.insert("content");
    e.act(Action::Quit);
    let s = draw(&mut e, 24, 18);
    let find = |text: &str| (0..18).find(|&y| row(&s, y).contains(text)).unwrap();
    assert!(find("Discard") < find("Cancel"));
    assert!(find("Cancel") < find("Save [s]"));
    assert!(matches!(key(&mut e, K::Esc), Outcome::FocusReleased(_)));
    assert!(e.buffer.dirty());
}

#[test]
fn cancelling_filename_after_save_and_quit_does_not_quit_on_a_later_save() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Editor::new(Buffer::new(""));
    e.buffer.insert("keep");
    e.act(Action::Quit);
    key(&mut e, K::Char('s'));
    key(&mut e, K::Esc);
    e.act(Action::Save);
    e.handle(Event::Paste(
        dir.path().join("later.txt").to_string_lossy().into_owned(),
    ));
    let Outcome::SaveAsRequested(path) = key(&mut e, K::Enter) else {
        panic!("Expected save path");
    };
    let result = e.buffer.save_as(&path);
    assert_eq!(e.save_finished(result), Outcome::Handled);
}

#[test]
fn delete_dialog_shows_path_danger_action_and_safe_default_at_narrow_widths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delete-me.txt");
    std::fs::write(&path, "keep until confirmed").unwrap();
    for width in [32, 80] {
        let mut e = Editor::new(Buffer::open(&path).unwrap());
        e.act(Action::DeleteFile);
        let s = draw(&mut e, width, 30);
        let text = screen(&s);
        assert!(text.contains("Delete file"));
        assert!(text.contains("Esc"));
        assert!(text.contains("Cancel"));
        let danger = s
            .content
            .iter()
            .any(|c| c.symbol() == "D" && c.fg == Color::Red);
        assert!(danger, "{text}");
        assert_eq!(key(&mut e, K::Enter), Outcome::Handled);
        assert!(path.exists());
    }
}

#[test]
fn document_switch_reuses_save_discard_cancel_without_quitting_or_losing_edits() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("current.md");
    std::fs::write(&path, "original").unwrap();
    let mut e = Editor::new(Buffer::open(&path).unwrap());
    e.buffer.insert("changed ");
    assert_eq!(e.request_document_switch(), Outcome::Handled);
    assert!(screen(&draw(&mut e, 80, 24)).contains("opening another document"));
    key(&mut e, K::Esc);
    assert!(!e.document_switch_pending());
    assert!(e.buffer.dirty());
    e.request_document_switch();
    assert_eq!(key(&mut e, K::Char('s')), Outcome::SaveRequested);
    assert_eq!(
        e.save_finished(Err(std::io::Error::other("test failure"))),
        Outcome::Handled
    );
    assert!(e.buffer.dirty());
    assert!(!e.document_switch_pending());
    e.request_document_switch();
    assert_eq!(key(&mut e, K::Char('s')), Outcome::SaveRequested);
    let saved = e.buffer.save();
    assert_eq!(e.save_finished(saved), Outcome::DocumentSwitchReady);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "changed original");
    e.buffer.insert("discard ");
    e.request_document_switch();
    assert_eq!(key(&mut e, K::Char('d')), Outcome::DocumentSwitchReady);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "changed original");
    assert!(
        e.buffer.dirty(),
        "host retains buffer until destination load succeeds"
    );
}

#[test]
fn untitled_switch_save_as_cancel_and_success() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("saved.md");
    let mut e = Editor::new(Buffer::new(""));
    e.buffer.insert("keep me");
    e.request_document_switch();
    key(&mut e, K::Char('s'));
    key(&mut e, K::Esc);
    assert!(e.buffer.dirty());
    assert!(!e.document_switch_pending());
    e.request_document_switch();
    key(&mut e, K::Char('s'));
    e.handle(Event::Paste(path.display().to_string()));
    assert_eq!(
        key(&mut e, K::Enter),
        Outcome::SaveAsRequested(path.clone())
    );
    let result = e.buffer.save_as(&path);
    assert_eq!(e.save_finished(result), Outcome::DocumentSwitchReady);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "keep me");
}
