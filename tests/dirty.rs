use editio::{
    Editor,
    buffer::{Buffer, Cursor},
};
use std::time::{Duration, Instant};

fn settle(b: &mut Buffer) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while b.dirty_check_pending() {
        assert!(Instant::now() < deadline);
        b.poll_dirty();
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn manual_reverts_are_clean_without_rewriting_revision_or_undo_history() {
    let mut b = Buffer::new("cat 猫🙂\n");
    let revision = b.revision();
    b.insert("x");
    assert!(b.dirty());
    assert!(!b.dirty_check_pending()); // Different byte lengths need no scan.
    b.cursors = vec![Cursor { anchor: 0, head: 1 }];
    b.insert("");
    let reverted_revision = b.revision();
    assert_ne!(reverted_revision, revision);
    assert!(b.dirty()); // Conservative while the worker has not verified equality.
    settle(&mut b);
    assert!(!b.dirty());
    assert_eq!(b.revision(), reverted_revision);
    b.undo();
    assert!(b.dirty());
    b.redo();
    settle(&mut b);
    assert!(!b.dirty());
    b.cursors = vec![Cursor { anchor: 0, head: 3 }];
    b.insert("dog");
    settle(&mut b);
    assert!(b.dirty()); // Same length does not mean same contents.
}
#[test]
fn saved_baselines_formats_and_stale_checks_remain_correct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(&path, "\u{feff}one\r\n").unwrap();
    let mut b = Buffer::open(&path).unwrap();
    assert!(!b.dirty());
    b.crlf = false;
    assert!(b.dirty());
    b.crlf = true;
    b.bom = false;
    assert!(b.dirty());
    b.bom = true;
    b.cursors = vec![Cursor { anchor: 0, head: 3 }];
    b.insert("two");
    b.poll_dirty(); // Schedule a check against the old saved baseline.
    b.save().unwrap();
    settle(&mut b);
    assert!(!b.dirty());
    b.cursors = vec![Cursor { anchor: 0, head: 3 }];
    b.insert("one");
    settle(&mut b);
    assert!(b.dirty()); // The original file is no longer the saved baseline.
    b.cursors = vec![Cursor { anchor: 0, head: 3 }];
    b.insert("two");
    b.poll_dirty();
    b.cursors = vec![Cursor { anchor: 0, head: 3 }];
    b.insert("six"); // Invalidates the pending equality result.
    settle(&mut b);
    assert!(b.dirty());
}
#[test]
fn large_files_skip_checks_and_editor_polls_supported_files_without_idle_work() {
    let mut b = Buffer::new(&"a".repeat(16 * 1024 * 1024 + 1));
    b.insert("x");
    b.cursors = vec![Cursor { anchor: 0, head: 1 }];
    b.insert("");
    assert!(b.dirty());
    assert!(!b.dirty_check_pending());
    let mut e = Editor::new(Buffer::new("same"));
    assert!(e.next_wakeup().is_none());
    e.buffer.insert("x");
    e.buffer.cursors = vec![Cursor { anchor: 0, head: 1 }];
    e.buffer.insert("");
    assert!(e.next_wakeup().is_some());
    let deadline = Instant::now() + Duration::from_secs(5);
    while e.has_background_work() {
        assert!(Instant::now() < deadline);
        e.poll_background();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!e.buffer.dirty());
    assert!(e.next_wakeup().is_none());
}
