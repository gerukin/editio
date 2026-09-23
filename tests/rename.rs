use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use editio::{Action, Editor, Mode, Outcome, PromptKind, buffer::Buffer};

#[test]
fn rename_moves_disk_bytes_and_retains_unsaved_edits_and_undo() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("old.md");
    let target = dir.path().join("new/nested/new.md");
    let original = "\u{feff}hello\r\nworld\r\n";
    std::fs::write(&source, original).unwrap();
    let mut b = Buffer::open(&source).unwrap();
    b.insert("edited ");
    let cursor = b.cursors.clone();
    b.rename_to(&target).unwrap();
    assert!(!source.exists());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), original);
    assert!(b.dirty());
    assert_eq!(b.cursors, cursor);
    b.undo();
    assert!(!b.dirty());
    b.redo();
    b.save().unwrap();
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "\u{feff}edited hello\r\nworld\r\n"
    );
    assert!(!source.exists());
}

#[test]
fn rename_collision_and_missing_source_preserve_files_and_buffer() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("old.txt");
    let target = dir.path().join("existing.txt");
    std::fs::write(&source, "original").unwrap();
    std::fs::write(&target, "keep me").unwrap();
    let mut b = Buffer::open(&source).unwrap();
    assert!(b.rename_to(&target).is_err());
    assert_eq!(b.path.as_ref(), Some(&source));
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "original");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep me");
    std::fs::remove_file(&source).unwrap();
    assert!(b.rename_to(&dir.path().join("missing/new")).is_err());
    assert_eq!(b.path.as_ref(), Some(&source));
}

#[test]
fn rename_preserves_external_changes_and_save_conflict_protection() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("old.txt");
    let target = dir.path().join("new.txt");
    std::fs::write(&source, "original").unwrap();
    let mut b = Buffer::open(&source).unwrap();
    b.insert("unsaved ");
    std::fs::write(&source, "external edit").unwrap();
    b.rename_to(&target).unwrap();
    assert!(b.save().is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "external edit");
    assert_eq!(b.text.to_string(), "unsaved original");
}

#[test]
fn rename_dialog_in_both_modes_returns_host_intent_and_cancels_safely() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("old.txt");
    std::fs::write(&source, "original").unwrap();
    for mode in [Mode::View, Mode::Edit] {
        let mut e = Editor::new(Buffer::open(&source).unwrap());
        e.mode = mode;
        e.act(Action::Rename);
        assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::Rename);
        e.handle(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(e.prompt.is_none());
        assert_eq!(e.mode, mode);
        e.act(Action::Rename);
        let p = e.prompt.as_mut().unwrap();
        p.value = "nested/new.txt".into();
        p.cursor = p.value.len();
        assert_eq!(
            e.handle(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE
            ))),
            Outcome::RenameRequested("nested/new.txt".into())
        );
        assert!(source.exists()); // only the host performs filesystem operations
        assert_eq!(e.mode, mode);
    }
    let mut e = Editor::new(Buffer::new("untitled"));
    e.act(Action::Rename);
    assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::SaveAs);
}

#[test]
fn relative_move_creates_directories() {
    let dir = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let source = dir.path().join("source");
    std::fs::write(&source, "text").unwrap();
    let target = dir.path().join("relative/destination");
    let relative = target
        .strip_prefix(std::env::current_dir().unwrap())
        .unwrap();
    let mut b = Buffer::open(&source).unwrap();
    b.rename_to(relative).unwrap();
    assert_eq!(b.path.as_deref(), Some(relative));
    b.insert("new ");
    b.save().unwrap();
    assert_eq!(std::fs::read_to_string(target).unwrap(), "new text");
}

#[cfg(unix)]
#[test]
fn rename_refuses_symlinks_and_dangling_destinations() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let link = dir.path().join("link");
    let dangling = dir.path().join("dangling");
    std::fs::write(&source, "original").unwrap();
    symlink(&source, &link).unwrap();
    symlink(dir.path().join("absent"), &dangling).unwrap();
    let mut b = Buffer::open(&link).unwrap();
    assert!(b.rename_to(&dir.path().join("destination")).is_err());
    let mut b = Buffer::open(&source).unwrap();
    assert!(b.rename_to(&dangling).is_err());
    assert_eq!(std::fs::read_to_string(source).unwrap(), "original");
    assert!(
        std::fs::symlink_metadata(dangling)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn cross_device_move_when_test_mount_is_provided() {
    let Some(root) = std::env::var_os("EDITIO_TEST_CROSS_DEVICE_ROOT") else {
        return;
    };
    let source_dir = tempfile::tempdir_in(root).unwrap();
    let target_dir = tempfile::tempdir().unwrap();
    let source = source_dir.path().join("original.txt");
    let target = target_dir.path().join("new/note.txt");
    let text = "original\n".repeat(100_000);
    std::fs::write(&source, &text).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            std::fs::metadata(source_dir.path()).unwrap().dev(),
            std::fs::metadata(target_dir.path()).unwrap().dev()
        );
    }
    let mut b = Buffer::open(&source).unwrap();
    b.insert("unsaved ");
    b.rename_to(&target).unwrap();
    assert!(!source.exists());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), text);
    assert!(b.dirty());
    b.save().unwrap();
    assert_eq!(
        std::fs::read_to_string(target).unwrap(),
        format!("unsaved {text}")
    );
}
