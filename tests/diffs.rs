use crossterm::event::{Event, KeyCode as K, KeyEvent, KeyModifiers as M};
use editio::{
    Action, Editor, Mode,
    buffer::{Buffer, Cursor},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer as Screen,
    style::{Color, Modifier},
};
use std::time::{Duration, Instant};
use tapp_ui::renderers::{
    RenderOptions, diff,
    preview::{Point, Selection},
};
fn screen(e: &mut Editor, w: u16, h: u16) -> Screen {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        e.poll_background();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        if !e.has_background_work() {
            break;
        }
        assert!(Instant::now() < until, "background did not settle");
        std::thread::sleep(Duration::from_millis(2));
    }
    t.backend().buffer().clone()
}
fn text(s: &Screen) -> String {
    (0..s.area.height - 1)
        .map(|y| {
            (0..s.area.width)
                .map(|x| s[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn key(e: &mut Editor, k: K, m: M) {
    e.handle(Event::Key(KeyEvent::new(k, m)));
}
fn compared(old: &str, new: &str, path: &str) -> Editor {
    let mut b = Buffer::new(new);
    b.path = Some(path.into());
    let mut e = Editor::new(b);
    e.compare_with(&Buffer::new(old)).unwrap();
    e
}

#[test]
fn malformed_hunks_remain_literal_and_unicode_lines_match_editor_positions() {
    let invalid = "--- a\n+++ b\n@@ -1,9 +1,9 @@\n-old\n+new\n";
    assert_eq!(diff::project(invalid, "diff").text, invalid);
    let eof = "--- a\n+++ b\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n";
    let projected = diff::project(eof, "diff");
    assert!(!projected.text.contains("-old"));
    assert!(projected.text.contains("\nnew\n"));
    assert_eq!(projected.raw[5], Some(diff::Kind::Added));
    for separator in [
        "\n", "\r\n", "\r", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}",
    ] {
        let old = format!("same{separator}old{separator}end");
        let new = format!("same{separator}new{separator}end");
        let a = diff::analyze(&old, &new).unwrap();
        assert_eq!(a.changes.len(), 1);
        assert_eq!(a.changes[0].old, 1..2);
        assert_eq!(a.changes[0].new, 1..2);
    }
}

#[test]
fn embedded_diff_markers_stay_inside_the_hosts_offset_rectangle() {
    let mut e = compared(
        "old\n",
        "replacement line long enough to wrap across several rows\n",
        "test.txt",
    );
    e.mode = Mode::View;
    e.wrap = true;
    let area = ratatui::layout::Rect::new(7, 3, 24, 10);
    let mut t = Terminal::new(TestBackend::new(45, 18)).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        e.poll_background();
        t.draw(|f| e.draw(f, area)).unwrap();
        if !e.has_background_work() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    let b = t.backend().buffer();
    assert!(
        (area.y..area.bottom() - 1)
            .filter(|&y| b[(area.right() - 1, y)].symbol() == "┃")
            .count()
            >= 2
    );
    for y in 0..18 {
        for x in 0..45 {
            if !area.contains((x, y).into()) {
                assert_eq!(b[(x, y)].symbol(), " ");
            }
        }
    }
}

#[test]
fn projection_preserves_current_fragments_raw_patch_and_copy() {
    let patch = "--- a.rs\n+++ b.rs\n@@ -1,2 +1,2 @@\n-let old = 1;\n+let new = 2;\n keep\n";
    for name in [None, Some("example.diff"), Some("example.patch")] {
        let mut b = Buffer::new(patch);
        b.path = name.map(Into::into);
        let mut e = Editor::new(b);
        let view = text(&screen(&mut e, 60, 16));
        assert!(e.rendered_view());
        assert!(view.contains("let new = 2;"), "{view}");
        assert!(!view.contains("let old"));
        assert!(!view.contains("+let"));
        key(&mut e, K::Char('['), M::NONE);
        assert_eq!(text(&screen(&mut e, 60, 16)), view);
        assert!(
            !e.command_contributions(1)
                .iter()
                .any(|c| c.available && c.label == "Reference version")
        );
        e.act(Action::ToggleMode);
        let edit = screen(&mut e, 60, 16);
        assert_eq!(e.mode, Mode::Edit);
        assert!(text(&edit).contains("-let old"));
        assert_eq!(e.buffer.text.to_string(), patch);
        e.act(Action::ToggleMode);
        screen(&mut e, 60, 16);
        assert!(e.rendered_view());
    }
    let p = diff::render(
        "--- a\n+++ b\n@@ -1 +1 @@\n-old\n+very long 日本語 content with words that wrap across lines\n",
        24,
        &RenderOptions {
            limit_width: true,
            ..Default::default()
        },
    );
    let start = p
        .diff
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .position(|r| r.source == 4)
        .unwrap();
    let copied = p.copy(&Selection {
        anchor: Point {
            row: start,
            column: 0,
        },
        head: Point {
            row: p.lines.len(),
            column: 0,
        },
        bounds: None,
    });
    assert_eq!(
        copied.trim_end_matches('\n'),
        "very long 日本語 content with words that wrap across lines"
    );
}
#[test]
fn side_switching_editing_undo_and_save_protect_reference() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.rs");
    let new = dir.path().join("new.rs");
    std::fs::write(&old, "let old = 1;\n").unwrap();
    std::fs::write(&new, "let new = 2;\n").unwrap();
    let mut e = Editor::new(Buffer::open(&new).unwrap());
    e.compare_with(&Buffer::open(&old).unwrap()).unwrap();
    let view = text(&screen(&mut e, 60, 12));
    assert!(view.contains("let new"));
    assert!(!view.contains("let old"));
    key(&mut e, K::Char('['), M::NONE);
    assert!(text(&screen(&mut e, 60, 12)).contains("let old"));
    key(&mut e, K::Char(']'), M::NONE);
    assert!(text(&screen(&mut e, 60, 12)).contains("let new"));
    key(&mut e, K::Char('['), M::NONE);
    screen(&mut e, 60, 12);
    e.act(Action::ToggleMode);
    screen(&mut e, 60, 12);
    e.act(Action::ToggleMode);
    assert!(text(&screen(&mut e, 60, 12)).contains("let new"));
    e.act(Action::ToggleMode);
    screen(&mut e, 60, 12);
    assert_eq!(e.buffer.text.to_string(), "let new = 2;\n");
    e.act(Action::SelectAll);
    e.paste("let unsaved = 3;\n");
    e.act(Action::ToggleMode);
    assert!(text(&screen(&mut e, 60, 12)).contains("let unsaved"));
    assert_eq!(std::fs::read_to_string(&new).unwrap(), "let new = 2;\n");
    e.buffer.save().unwrap();
    assert_eq!(std::fs::read_to_string(&old).unwrap(), "let old = 1;\n");
    assert_eq!(std::fs::read_to_string(&new).unwrap(), "let unsaved = 3;\n");
    e.act(Action::ToggleMode);
    screen(&mut e, 60, 12);
    e.act(Action::Undo);
    e.act(Action::ToggleMode);
    assert!(text(&screen(&mut e, 60, 12)).contains("let new"));
}
#[test]
fn ctrl_p_n_cycles_both_modes_and_arrows_are_not_change_commands() {
    let old = (0..100).map(|n| format!("line {n}\n")).collect::<String>();
    let new = old
        .replace("line 10\n", "changed ten\n")
        .replace("line 80\n", "changed eighty\n");
    let mut e = compared(&old, &new, "sample.txt");
    e.wrap = false;
    screen(&mut e, 60, 8);
    key(&mut e, K::Char('n'), M::CONTROL);
    let first = text(&screen(&mut e, 60, 8));
    let pos = e.scroll;
    key(&mut e, K::Char('n'), M::CONTROL);
    screen(&mut e, 60, 8);
    let second = e.scroll;
    assert_ne!(pos, second);
    key(&mut e, K::Char('n'), M::CONTROL);
    assert_eq!(text(&screen(&mut e, 60, 8)), first);
    key(&mut e, K::Char('p'), M::CONTROL);
    screen(&mut e, 60, 8);
    assert_eq!(e.scroll, second);
    for _ in 0..20 {
        e.act(Action::ToggleMode);
        screen(&mut e, 60, 8);
        e.act(Action::ToggleMode);
        screen(&mut e, 60, 8);
        assert_eq!(e.scroll, second);
    }
    key(&mut e, K::Down, M::CONTROL);
    screen(&mut e, 60, 8);
    assert_eq!(e.scroll, second + 1);
    e.act(Action::ToggleMode);
    screen(&mut e, 60, 8);
    e.buffer.cursors = vec![Cursor::at(0)];
    key(&mut e, K::Char('n'), M::CONTROL);
    assert_eq!(e.buffer.text.char_to_line(e.buffer.cursors[0].head), 10);
    key(&mut e, K::Char('n'), M::CONTROL);
    assert_eq!(e.buffer.text.char_to_line(e.buffer.cursors[0].head), 80);
    key(&mut e, K::Char('n'), M::CONTROL);
    assert_eq!(e.buffer.text.char_to_line(e.buffer.cursors[0].head), 10);
    e.act(Action::Find);
    key(&mut e, K::Char('p'), M::CONTROL);
    assert_eq!(e.buffer.text.char_to_line(e.buffer.cursors[0].head), 10);
}
#[test]
fn comparison_marker_meanings_survive_repeated_mode_switches() {
    for (old, new, row, color) in [
        ("same\n", "added\nsame\n", 0, Color::Green),
        ("removed\nsame\n", "same\n", 0, Color::Red),
        ("old\nsame\n", "new\nsame\n", 0, Color::Blue),
    ] {
        let mut e = compared(old, new, "test.rs");
        e.wrap = false;
        for _ in 0..4 {
            let rendered = screen(&mut e, 40, 8);
            assert_eq!(rendered[(39, row)].symbol(), "┃");
            assert_eq!(rendered[(39, row)].fg, color, "mode {:?}", e.mode);
            e.act(Action::ToggleMode);
        }
    }
}

#[test]
fn markers_only_preserve_syntax_and_cover_wrapped_rows_without_copying_borders() {
    let mut e = compared("let old = 123;\n", "let new = 456;\n", "test.rs");
    e.wrap = false;
    let view = screen(&mut e, 40, 8);
    assert_eq!(view[(39, 0)].symbol(), "┃");
    assert_eq!(view[(39, 0)].fg, Color::Blue);
    assert_ne!(view[(0, 0)].fg, Color::Blue);
    assert_eq!(view[(0, 0)].bg, Color::Reset);
    e.act(Action::ToggleMode);
    let edit = screen(&mut e, 40, 8);
    assert_eq!(edit[(39, 0)].fg, Color::Blue);
    let mut e = Editor::new(Buffer::new(
        "-old\n+replacement with a long sequence of words that must wrap across lines\n",
    ));
    e.limit_width = true;
    let view = screen(&mut e, 24, 12);
    let borders = (0..8).filter(|&y| view[(23, y)].symbol() == "┃").count();
    assert!(borders >= 3, "{}", text(&view));
    e.act(Action::ToggleMode);
    let edit = screen(&mut e, 24, 12);
    assert_eq!(edit[(0, 0)].fg, Color::Red);
    assert!(edit[(0, 0)].modifier.contains(Modifier::BOLD));
    assert_eq!(edit[(1, 0)].fg, Color::Reset);
}
#[test]
fn bounded_analysis_reconstructs_changes_and_preserves_eof_and_repeated_lines() {
    for size in [0, 1, 10, 100, 2000] {
        let old = (0..size).map(|n| format!("line {n}\n")).collect::<String>();
        for stride in [1, 3, 21] {
            let new = (0..size + 1)
                .filter(|n| n % stride != 0)
                .map(|n| format!("line {n}\ninsert {n}\n"))
                .collect::<String>();
            // Reconstruction is a correctness check, independent of debug-build speed.
            // Production guard behavior is checked separately with an expired deadline.
            let a = diff::analyze_with_budget(&old, &new, Duration::from_secs(2)).unwrap();
            let original: Vec<_> = old.split_inclusive('\n').collect();
            let target: Vec<_> = new.split_inclusive('\n').collect();
            let mut result = String::new();
            let mut at = 0;
            for c in a.changes {
                for s in &original[at..c.old.start] {
                    result.push_str(s);
                }
                for s in &target[c.new.clone()] {
                    result.push_str(s);
                }
                at = c.old.end;
            }
            for s in &original[at..] {
                result.push_str(s);
            }
            assert_eq!(result, new);
        }
    }
    assert!(!diff::analyze("a", "a\n").unwrap().changes.is_empty());
    assert!(diff::analyze_with_budget("a", "b", Duration::ZERO).is_err());
    assert!(diff::analyze(&"x".repeat(diff::MAX_BYTES + 1), "").is_err());
    let a = diff::analyze(&"old\n".repeat(10_000), &"new\n".repeat(10_000)).unwrap();
    assert!(a.coarse);
    assert_eq!(a.changes.len(), 1);
    let middle: String = (0..35_000)
        .map(|i| format!("record {}\n", i % 64))
        .collect();
    let other: String = (0..35_000)
        .map(|i| format!("record {}\n", (i * 17 + 13) % 64))
        .collect();
    let old = format!("unchanged prefix\n{middle}unchanged suffix\n");
    let new = format!("unchanged prefix\n{other}unchanged suffix\n");
    let a = diff::analyze_with_budget(&old, &new, Duration::from_secs(2)).unwrap();
    assert!(
        a.coarse,
        "large repetitive regions must bypass non-cancellable matching"
    );
    assert_eq!(a.changes[0].old, 1..35_001);
    assert_eq!(reconstruct(&old, &new, &a), new);
}

fn reconstruct(old: &str, new: &str, a: &diff::Analysis) -> String {
    let o: Vec<_> = old.split_inclusive('\n').collect();
    let n: Vec<_> = new.split_inclusive('\n').collect();
    let mut result = String::new();
    let mut at = 0;
    for c in &a.changes {
        assert!(c.old.start >= at);
        for s in &o[at..c.old.start] {
            result.push_str(s);
        }
        for s in &n[c.new.clone()] {
            result.push_str(s);
        }
        at = c.old.end;
    }
    for s in &o[at..] {
        result.push_str(s);
    }
    result
}
#[test]
fn incremental_windows_remain_correct_through_insert_delete_undo_and_eof_changes() {
    let old = (0..120).map(|n| format!("line {n}\n")).collect::<String>();
    let mut target = old.clone();
    let mut cached = diff::analyze(&old, &target).unwrap();
    let mut seed = 7u64;
    for step in 0..180 {
        let mut lines: Vec<_> = target.split_inclusive('\n').map(str::to_owned).collect();
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let at = seed as usize % (lines.len() + 1);
        match step % 4 {
            0 => lines.insert(at, format!("inserted {step}\n")),
            1 if at < lines.len() => {
                lines.remove(at);
            }
            2 if at < lines.len() => lines[at] = format!("changed {step}\n"),
            _ => {}
        }
        let new = lines.concat();
        let next = diff::update(&old, &target, &new, &cached).unwrap();
        assert_eq!(reconstruct(&old, &new, &next), new, "step {step}");
        target = new;
        cached = next;
    }
    let restored = diff::update(&old, &target, &old, &cached).unwrap();
    assert!(restored.changes.is_empty());
    for (a, b, c) in [
        ("", "a", ""),
        ("a\n", "b\n", "b"),
        ("x", "", "y"),
        ("a\na\na\n", "a\nb\na\n", "b\na\n"),
    ] {
        let first = diff::analyze(a, b).unwrap();
        let next = diff::update(a, b, c, &first).unwrap();
        assert_eq!(reconstruct(a, c, &next), c);
    }
}
#[test]
fn medium_edits_are_on_demand_large_files_stay_editable_and_plain_files_stay_idle() {
    let old = "same line\n".repeat(15_000);
    let mut e = compared(&old, &old, "medium.txt");
    // Exercise matching policy separately from the guarded full preview preparation.
    e.mode = Mode::Edit;
    screen(&mut e, 60, 10);
    e.buffer.insert("changed\n");
    let s = screen(&mut e, 60, 10);
    assert!(!(0..9).any(|y| s[(59, y)].symbol() == "┃"));
    assert!(!e.has_background_work());
    key(&mut e, K::Char('n'), M::CONTROL);
    let s = screen(&mut e, 60, 10);
    assert!((0..9).any(|y| s[(59, y)].symbol() == "┃"));
    let large = Buffer::new(&"x".repeat(diff::MAX_BYTES + 1));
    let mut ordinary = Editor::new(Buffer::new("plain text"));
    assert!(ordinary.compare_with(&large).is_err());
    ordinary.mode = Mode::Edit;
    ordinary.paste("still editable ");
    screen(&mut ordinary, 60, 10);
    assert!(!ordinary.has_background_work());
    assert!(!diff::inline_diff("- item\n+ another item\n"));
}
