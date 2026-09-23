use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use editio::{Action, Editor, Mode, Outcome, buffer::Buffer};
fn key(e: &mut Editor, k: KeyCode) -> Outcome {
    e.handle(Event::Key(KeyEvent::new(k, KeyModifiers::NONE)))
}

#[test]
fn delete_requires_explicit_confirmation_in_both_modes_and_keeps_text() {
    for mode in [Mode::View, Mode::Edit] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "original").unwrap();
        let mut e = Editor::new(Buffer::open(&path).unwrap());
        e.mode = mode;
        e.buffer.insert("unsaved ");
        let cursor = e.buffer.cursors.clone();
        assert_eq!(e.act(Action::DeleteFile), Outcome::Handled);
        assert_eq!(key(&mut e, KeyCode::Enter), Outcome::Handled); // Cancel is default
        assert!(path.exists());
        e.act(Action::DeleteFile);
        key(&mut e, KeyCode::Esc);
        assert!(path.exists());
        e.act(Action::DeleteFile);
        assert_eq!(key(&mut e, KeyCode::Char('y')), Outcome::Handled); // no generic yes shortcut
        assert!(path.exists());
        assert_eq!(
            key(&mut e, KeyCode::Char('d')),
            Outcome::DeleteFileRequested
        );
        assert!(path.exists()); // host owns deletion
        e.buffer.delete_file().unwrap();
        assert!(!path.exists());
        assert_eq!(e.mode, mode);
        assert!(e.buffer.path.is_none());
        assert_eq!(e.buffer.cursors, cursor);
        assert_eq!(e.buffer.text.to_string(), "unsaved original");
        e.buffer.undo();
        assert_eq!(e.buffer.text.to_string(), "original");
        assert!(e.buffer.dirty()); // even undo cannot discard the only remaining copy silently
        assert_eq!(e.act(Action::Quit), Outcome::Handled);
        key(&mut e, KeyCode::Esc);
        let recovered = dir.path().join("recovered.md");
        e.buffer.set_new_path(&recovered).unwrap();
        e.buffer.save().unwrap();
        assert!(!e.buffer.dirty());
        assert_eq!(std::fs::read_to_string(recovered).unwrap(), "original");
        assert!(!path.exists());
    }
}

#[test]
fn external_changes_and_missing_files_are_not_deleted_or_detached() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("note");
    std::fs::write(&path, "original").unwrap();
    let mut b = Buffer::open(&path).unwrap();
    std::fs::write(&path, "external").unwrap();
    assert!(b.delete_file().is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "external");
    assert_eq!(b.path.as_ref(), Some(&path));
    std::fs::remove_file(&path).unwrap();
    assert!(b.delete_file().is_err());
    assert_eq!(b.path.as_ref(), Some(&path));
    assert!(Buffer::new("untitled").delete_file().is_err());
}

#[cfg(unix)]
#[test]
fn symlink_deletion_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original");
    let link = dir.path().join("link");
    std::fs::write(&path, "original").unwrap();
    std::os::unix::fs::symlink(&path, &link).unwrap();
    let mut b = Buffer::open(&link).unwrap();
    assert!(b.delete_file().is_err());
    assert!(link.exists());
    assert!(path.exists());
}
