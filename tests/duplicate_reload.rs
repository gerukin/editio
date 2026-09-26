use crossterm::event::{Event, KeyCode as K, KeyEvent, KeyModifiers as M};
use editio::{
    Action, Editor, Mode, Outcome, PromptKind,
    buffer::{Buffer, Cursor},
};
use ratatui::{Terminal, backend::TestBackend};
fn key(e: &mut Editor, k: K) -> Outcome {
    e.handle(Event::Key(KeyEvent::new(k, M::NONE)))
}

#[test]
fn duplication_is_transactional_and_protects_original_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.md");
    std::fs::write(&original, "original").unwrap();
    let mut b = Buffer::open(&original).unwrap();
    b.insert("edited ");
    assert!(b.save_as(&original).is_err());
    std::fs::hard_link(&original, dir.path().join("alias.md")).unwrap();
    assert!(b.save_as(&dir.path().join("alias.md")).is_err());
    assert!(b.save_as(&original.join("impossible.md")).is_err());
    assert_eq!(b.path.as_ref(), Some(&original));
    assert!(b.dirty());
    b.undo();
    assert_eq!(b.text.to_string(), "original");
    b.redo();
    let copy = dir.path().join("new/folder/copy.md");
    b.save_as(&copy).unwrap();
    assert_eq!(b.path.as_ref(), Some(&copy));
    assert!(!b.dirty());
    assert_eq!(std::fs::read_to_string(&original).unwrap(), "original");
    assert_eq!(std::fs::read_to_string(copy).unwrap(), "edited original");
}

#[test]
fn duplicate_choices_cancel_save_first_or_keep_changes_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original.md");
    std::fs::write(&path, "original").unwrap();
    for mode in [Mode::View, Mode::Edit] {
        let mut e = Editor::new(Buffer::open(&path).unwrap());
        e.mode = mode;
        e.buffer.insert("new ");
        e.act(Action::SaveAs);
        key(&mut e, K::Esc);
        assert!(e.prompt.is_none());
        assert!(e.buffer.dirty());
        e.act(Action::SaveAs);
        key(&mut e, K::Enter); // safe default: take changes to copy
        assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::SaveAs);
        assert_eq!(e.prompt.as_ref().unwrap().value, path.to_string_lossy());
        key(&mut e, K::Esc);
        e.act(Action::SaveAs);
        key(&mut e, K::Right); // wraps to save original first
        assert_eq!(key(&mut e, K::Enter), Outcome::SaveRequested);
        let result = e.buffer.save();
        e.save_finished(result);
        assert!(!e.buffer.dirty());
        assert!(e.prompt.is_some());
        assert_eq!(e.mode, mode);
    }
}

#[test]
fn detached_copy_keeps_undo_and_cannot_save_back_to_deleted_origin() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original.md");
    std::fs::write(&path, "original").unwrap();
    let mut e = Editor::new(Buffer::open(&path).unwrap());
    e.buffer.insert("edits ");
    e.detach_copy();
    assert!(
        e.is_markdown(),
        "unsaved copy retains its source file format"
    );
    assert!(e.buffer.path.is_none());
    assert!(e.buffer.dirty());
    e.buffer.undo();
    assert_eq!(e.buffer.text.to_string(), "original");
    assert!(e.buffer.dirty()); // must be saved independently, even after undo
    std::fs::remove_file(&path).unwrap();
    assert!(e.buffer.save_as(&path).is_err());
    e.act(Action::Save);
    assert_eq!(e.prompt.as_ref().unwrap().value, path.to_string_lossy());
}

#[test]
fn reload_preserves_mode_selection_and_viewport_without_revision_collision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.txt");
    let text: String = (0..100).map(|i| format!("Unique line {i}\n")).collect();
    std::fs::write(&path, &text).unwrap();
    for mode in [Mode::View, Mode::Edit] {
        std::fs::write(&path, &text).unwrap();
        let mut e = Editor::new(Buffer::open(&path).unwrap());
        e.mode = mode;
        e.wrap = false;
        let pos = e.buffer.text.line_to_char(40);
        e.buffer.cursors = vec![Cursor {
            anchor: pos,
            head: pos + 6,
        }];
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let revision = e.buffer.revision();
        std::fs::write(&path, format!("Inserted line\n{text}")).unwrap();
        e.reload_disk(Buffer::open(&path).unwrap());
        t.draw(|f| e.draw(f, f.area())).unwrap();
        assert_eq!(e.mode, mode);
        assert!(!e.buffer.dirty());
        assert_ne!(e.buffer.revision(), revision);
        assert_eq!(e.buffer.selected(), "Unique");
        assert_eq!(e.buffer.text.char_to_line(e.buffer.cursors[0].anchor), 41);
    }
}

#[test]
fn reload_refreshes_find_and_retains_selected_occurrence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.txt");
    let original = "Earlier needle in this paragraph with some extra context.\n\nLater needle in another paragraph with different context.\n";
    std::fs::write(&path, original).unwrap();
    let mut e = Editor::new(Buffer::open(&path).unwrap());
    e.mode = Mode::Edit;
    let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
    t.draw(|f| e.draw(f, f.area())).unwrap();
    e.act(Action::Find);
    e.handle(Event::Paste("needle".into()));
    t.draw(|f| e.draw(f, f.area())).unwrap();
    key(&mut e, K::Down);
    assert_eq!(e.prompt.as_ref().unwrap().selected, 1);
    std::fs::write(
        &path,
        format!("New needle added well before existing matches and their context.\n{original}"),
    )
    .unwrap();
    e.reload_disk(Buffer::open(&path).unwrap());
    t.draw(|f| e.draw(f, f.area())).unwrap();
    assert_eq!(e.prompt.as_ref().unwrap().value, "needle");
    assert_eq!(e.prompt.as_ref().unwrap().selected, 2);
    assert_eq!(e.mode, Mode::Edit);
}

#[test]
fn background_read_limit_preserves_original_and_unchanged_reads_skip_parsing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("growing.txt");
    std::fs::write(&path, "initial").unwrap();
    let b = Buffer::open(&path).unwrap();
    assert!(
        Buffer::read_changed(&path, &b.disk_baseline(), 7)
            .unwrap()
            .is_none()
    );
    std::fs::write(&path, "much larger external file").unwrap();
    assert!(Buffer::read_changed(&path, &b.disk_baseline(), 7).is_err());
    assert_eq!(b.text.to_string(), "initial");
    assert_eq!(
        Buffer::read_changed(&path, &b.disk_baseline(), 1024)
            .unwrap()
            .unwrap()
            .text
            .to_string(),
        "much larger external file"
    );
}
