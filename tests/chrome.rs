use crossterm::event::{
    Event, KeyCode as K, KeyEvent, KeyModifiers as M, MouseButton as B, MouseEvent,
    MouseEventKind as MK,
};
use editio::{Action, Editor, Mode, Outcome, buffer::Buffer};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer as Screen,
    style::{Color, Modifier},
};
fn editor(text: &str, path: &str) -> Editor {
    let mut b = Buffer::new(text);
    b.path = Some(path.into());
    Editor::new(b)
}
fn key(e: &mut Editor, code: K, modifiers: M) -> Outcome {
    e.handle(Event::Key(KeyEvent::new(code, modifiers)))
}
fn draw(e: &mut Editor, width: u16, height: u16) -> Screen {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
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
fn compact_bar_has_no_header_and_keeps_help_and_total_without_numbers() {
    let mut e = editor(
        "hello\nworld",
        "a-very-long-file-name-that-needs-shortening.txt",
    );
    for width in [16, 32, 48, 80] {
        let s = draw(&mut e, width, 10);
        assert_eq!(row(&s, 0).trim_end(), "hello");
        let status = row(&s, 9);
        assert!(status.ends_with("Ctrl+?"));
        assert!(status.contains("100%/2"), "{status}");
        assert!(
            !s[(0, 9)]
                .modifier
                .intersects(Modifier::REVERSED | Modifier::DIM)
        );
        assert_eq!(s[(0, 9)].fg, Color::Black);
        assert_eq!(s[(0, 9)].bg, Color::Blue);
        if width == 48 {
            assert!(status.contains('…'));
        }
        assert!(!status.contains("Ctrl+S") && !status.contains("Ctrl+F"));
    }
}
#[test]
fn command_center_help_and_info_are_accessible_and_modal() {
    let mut e = editor("hello", "example.txt");
    e.mode = Mode::Edit;
    key(&mut e, K::Char('k'), M::CONTROL);
    key(&mut e, K::Tab, M::NONE);
    assert!(!e.options);
    let s = draw(&mut e, 80, 24);
    assert!(
        s.content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
            .contains("example.txt")
    );
    e.handle(Event::Paste("help".into()));
    key(&mut e, K::Enter, M::NONE);
    let s = draw(&mut e, 80, 24);
    assert!((0..24).any(|y| row(&s, y).contains(" Help ")));
    key(&mut e, K::Char('x'), M::NONE);
    e.handle(Event::Paste("should not edit".into()));
    assert_eq!(e.buffer.text.to_string(), "hello");
    key(&mut e, K::End, M::NONE);
    draw(&mut e, 30, 10);
    key(&mut e, K::Esc, M::NONE);
    draw(&mut e, 80, 24);
    e.handle(Event::Mouse(MouseEvent {
        kind: MK::Down(B::Left),
        column: 76,
        row: 23,
        modifiers: M::NONE,
    }));
    let s = draw(&mut e, 80, 24);
    assert!((0..24).any(|y| row(&s, y).contains(" Help ")));
    key(&mut e, K::F(1), M::NONE);
    key(&mut e, K::Char('?'), M::CONTROL | M::SHIFT);
    let s = draw(&mut e, 80, 24);
    assert!((0..24).any(|y| row(&s, y).contains(" Help ")));
    let handoff = KeyEvent::new(K::Enter, M::CONTROL);
    e.handoff_keys.push(handoff);
    assert_eq!(
        e.handle(Event::Key(handoff)),
        Outcome::FocusReleased(handoff)
    );
}
#[test]
fn numbers_toggle_via_palette_and_leave_selection_in_source_coordinates() {
    let mut keys = std::collections::HashSet::new();
    assert!(editio::COMMANDS.iter().all(|c| keys.insert(c.key)));
    let mut e = editor("alpha\nbeta\ngamma", "lines.txt");
    e.mode = Mode::Edit;
    key(&mut e, K::Char('k'), M::CONTROL);
    key(&mut e, K::Tab, M::NONE);
    e.handle(Event::Paste("line numbers".into()));
    key(&mut e, K::Enter, M::NONE);
    assert!(e.line_numbers);
    let s = draw(&mut e, 40, 10);
    assert!(row(&s, 0).starts_with("1 alpha"));
    assert!(row(&s, 9).contains("1/3:1/6"));
    e.handle(Event::Mouse(MouseEvent {
        kind: MK::Down(B::Left),
        column: 3,
        row: 1,
        modifiers: M::NONE,
    }));
    assert_eq!(e.buffer.cursors[0].head, 7);
    key(&mut e, K::F(2), M::NONE);
    for ch in "line numbers".chars() {
        key(&mut e, K::Char(ch), M::NONE);
    }
    key(&mut e, K::Enter, M::NONE);
    assert!(!e.line_numbers);
    assert!(row(&draw(&mut e, 40, 10), 9).contains("2/3:2/5"));
    e.act(Action::ToggleLineNumbers);
    e.act(Action::SelectAll);
    assert_eq!(
        e.act(Action::Copy),
        Outcome::CopyRequested("alpha\nbeta\ngamma".into())
    );
    assert!(row(&draw(&mut e, 16, 10), 0).starts_with("1 alpha"));
}
#[test]
fn source_and_preview_stop_at_last_full_viewport() {
    let source = (0..40)
        .map(|i| format!("line{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    for file in ["lines.txt", "lines.rs", "lines.md"] {
        let mut e = editor(&source, file);
        if file.ends_with("md") {
            e.source = true;
        }
        draw(&mut e, 40, 8);
        e.scroll = 1000;
        let s = draw(&mut e, 40, 8);
        assert_eq!(e.scroll, 33);
        assert!(row(&s, 6).contains("line39"));
        e.act(Action::ToggleLineNumbers);
        e.scroll = 1000;
        assert!(row(&draw(&mut e, 40, 8), 6).contains("line39"));
    }
    let mut e = editor(&source.replace('\n', "\n\n"), "preview.md");
    draw(&mut e, 40, 8);
    key(&mut e, K::End, M::NONE);
    let s = draw(&mut e, 40, 8);
    assert!(row(&s, 6).contains("line39"));
    e.act(Action::ToggleLineNumbers);
    draw(&mut e, 40, 8);
    key(&mut e, K::End, M::NONE);
    assert!(row(&draw(&mut e, 40, 8), 6).contains("line39"));
}

#[test]
fn modals_share_equal_margins_padding_and_palette_filters_immediately() {
    use ratatui::layout::Rect;
    for (width, height) in [(80, 24), (81, 25), (120, 42), (31, 11)] {
        let mut e = editor("unchanged", "notes.txt");
        let mut t = Terminal::new(TestBackend::new(width + 8, height + 6)).unwrap();
        let area = Rect::new(4, 3, width, height);
        let mut corners = Vec::new();
        for help in [false, true] {
            key(&mut e, K::Esc, M::NONE);
            if help {
                e.act(Action::Help);
            } else {
                key(&mut e, K::Char('k'), M::CONTROL);
            }
            t.draw(|f| e.draw(f, area)).unwrap();
            let screen = t.backend().buffer();
            let points: Vec<_> = (area.y..area.bottom())
                .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
                .filter(|&(x, y)| matches!(screen[(x, y)].symbol(), "┌" | "┐" | "└" | "┘"))
                .collect();
            assert_eq!(points.len(), 4);
            let (left, top) = points[0];
            let (right, bottom) = points[3];
            assert_eq!(left - area.x, area.right() - right - 1);
            assert!((top - area.y).abs_diff(area.bottom() - bottom - 2) <= u16::from(help));
            assert_eq!(screen[(left + 1, top + 1)].symbol(), " ");
            assert_eq!(screen[(right - 1, top + 1)].symbol(), " ");
            corners.push(points);
        }
        assert_eq!(corners[0][0].0, corners[1][0].0);
    }
    let mut e = editor("unchanged", "notes.txt");
    key(&mut e, K::Char('k'), M::CONTROL);
    assert_eq!(e.prompt.as_ref().unwrap().value, "");
    for c in "toggle line numbers".chars() {
        key(&mut e, K::Char(c), M::NONE);
    }
    assert_eq!(e.matching_commands().len(), 1);
    key(&mut e, K::Enter, M::NONE);
    assert!(e.line_numbers);
    assert_eq!(e.buffer.text.to_string(), "unchanged");
    key(&mut e, K::Char('k'), M::CONTROL);
    for c in "save".chars() {
        key(&mut e, K::Char(c), M::NONE);
    }
    let s = draw(&mut e, 80, 24);
    let text = (0..23).map(|y| row(&s, y)).collect::<String>();
    assert!(!text.contains(" · "));
    let (x, y) = (0..23)
        .find_map(|y| row(&s, y).find("Ctrl+S").map(|x| (x as u16, y)))
        .unwrap();
    assert_eq!(s[(x, y)].fg, Color::Cyan);
}

#[test]
fn command_shortcut_precedes_all_editor_modes_and_dialogs() {
    use crossterm::event::{KeyEventKind, KeyEventState};
    use editio::{Prompt, PromptKind};
    for mode in [Mode::View, Mode::Edit] {
        for source in [false, true] {
            for kind in [
                None,
                Some(PromptKind::Find),
                Some(PromptKind::Fuzzy),
                Some(PromptKind::Goto),
                Some(PromptKind::ReplaceFind),
                Some(PromptKind::ReplaceWith),
                Some(PromptKind::SaveAs),
            ] {
                let mut e = editor("unchanged", "notes.md");
                e.mode = mode;
                e.source = source;
                e.prompt = kind.map(|kind| Prompt {
                    kind,
                    value: "unfinished".into(),
                    cursor: 10,
                    selected: 0,
                });
                // Enhanced terminal events may carry state flags or repeat events.
                let event = KeyEvent {
                    code: K::Char('k'),
                    modifiers: M::CONTROL,
                    kind: KeyEventKind::Repeat,
                    state: KeyEventState::CAPS_LOCK,
                };
                assert_eq!(e.handle(Event::Key(event)), Outcome::Handled);
                assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::Palette);
                assert_eq!(e.mode, mode);
                assert_eq!(e.source, source);
                assert_eq!(e.buffer.text.to_string(), "unchanged");
                key(&mut e, K::Char('s'), M::NONE);
                key(&mut e, K::Char('k'), M::CONTROL);
                assert_eq!(e.prompt.as_ref().unwrap().value, "s");
            }
        }
    }
    for shortcut in [K::Char('k'), K::F(2)] {
        let modifiers = if shortcut == K::F(2) {
            M::NONE
        } else {
            M::CONTROL
        };
        let mut e = editor("unchanged", "notes.txt");
        e.act(Action::Help);
        key(&mut e, shortcut, modifiers);
        assert!(row(&draw(&mut e, 80, 24), 1).contains("Command center"));
        key(&mut e, K::Tab, M::NONE);
        assert!(!e.options);
        key(&mut e, shortcut, modifiers);
        assert!(!e.options);
        assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::Palette);
    }
    let mut e = editor("unchanged", "notes.txt");
    e.handoff_keys.push(KeyEvent::new(K::Char('k'), M::CONTROL));
    assert!(matches!(
        key(&mut e, K::Char('k'), M::CONTROL),
        Outcome::FocusReleased(_)
    ));
    assert!(e.prompt.is_none());
}

#[test]
fn modal_titles_have_separate_right_aligned_escape_hints() {
    use editio::{Prompt, PromptKind};
    for (kind, title) in [
        (PromptKind::Palette, "Command center"),
        (PromptKind::Find, "Find"),
        (PromptKind::Goto, "Go to line"),
        (PromptKind::SaveAs, "Save as"),
        (PromptKind::ReplaceFind, "Replace: find"),
        (PromptKind::ReplaceWith, "Replace all with"),
    ] {
        let mut e = editor("hello", "file.txt");
        e.prompt = Some(Prompt {
            kind,
            value: String::new(),
            cursor: 0,
            selected: 0,
        });
        for (w, h) in [(80, 24), (51, 15), (32, 10)] {
            let s = draw(&mut e, w, h);
            let border = (0..h)
                .map(|y| row(&s, y))
                .find(|s| s.contains('┌'))
                .unwrap();
            assert!(border.contains(title), "{border}");
            assert!(
                border.ends_with(" Esc ┐  ") || border.trim_end().ends_with(" Esc ┐"),
                "{border}"
            );
        }
    }
}

#[test]
fn help_path_is_copyable_with_muted_separators_and_colored_statistics() {
    let mut e = editor("hello\n猫", "C:\\notes\\file.md");
    e.act(Action::Info);
    let screen = draw(&mut e, 100, 40);
    let (x, y) = (0..40)
        .find_map(|y| {
            (0..100)
                .find(|&x| screen[(x, y)].symbol() == "\\")
                .map(|x| (x, y))
        })
        .unwrap();
    assert_eq!(screen[(x, y)].fg, Color::DarkGray);
    assert_eq!(
        e.handle(Event::Mouse(MouseEvent {
            kind: MK::Down(B::Left),
            column: x,
            row: y,
            modifiers: M::NONE
        })),
        Outcome::CopyRequested(
            std::path::absolute(e.buffer.path.as_ref().unwrap())
                .unwrap()
                .display()
                .to_string()
        )
    );
    let label_row = (0..40).find(|&y| row(&screen, y).contains("Size")).unwrap();
    let label_x = row(&screen, label_row).find("Size").unwrap() as u16;
    assert_eq!(screen[(label_x, label_row)].fg, Color::Reset);
    assert!(
        screen
            .content
            .iter()
            .any(|c| c.symbol() == "2" && c.fg == Color::Blue)
    );
}

#[test]
fn palette_filters_context_and_uses_centered_native_list_scrolling() {
    let mut e = editor("hello", "file.md");
    key(&mut e, K::Char('k'), M::CONTROL);
    let labels = e
        .matching_commands()
        .iter()
        .map(|c| c.label)
        .collect::<Vec<_>>();
    assert!(!labels.iter().any(|l| l.starts_with("Delete")));
    assert!(labels.iter().any(|l| l.starts_with("Render / source")));
    e.mode = Mode::Edit;
    let labels = e
        .matching_commands()
        .iter()
        .map(|c| c.label)
        .collect::<Vec<_>>();
    assert!(labels.iter().any(|l| l.starts_with("Delete")));
    assert!(!labels.iter().any(|l| l.starts_with("Render / source")));
    draw(&mut e, 80, 16);
    for _ in 0..15 {
        key(&mut e, K::Down, M::NONE);
        draw(&mut e, 80, 16);
    }
    let s = draw(&mut e, 80, 16);
    let selected_y = (0..16)
        .find(|&y| {
            (0..s.area.width).any(|x| {
                s[(x, y)].bg == tapp_ui::theme::terminal_selection_background()
                    && s[(x, y)].modifier.contains(ratatui::style::Modifier::BOLD)
            })
        })
        .unwrap();
    assert!((6..=9).contains(&selected_y), "selected row {selected_y}");
    for _ in 0..100 {
        key(&mut e, K::Down, M::NONE);
    }
    let s = draw(&mut e, 80, 16);
    assert!((3..13).all(|y| !row(&s, y).trim_matches(['│', ' ']).is_empty()));
    key(&mut e, K::Tab, M::NONE);
    assert!(!e.options);
    assert!(e.prompt.is_some());
}

#[test]
fn find_previews_click_jump_and_keep_state_across_escape() {
    let mut e = editor("before 猫 after\nnext 猫 last", "file.txt");
    key(&mut e, K::Char('f'), M::NONE);
    e.handle(Event::Paste("猫".into()));
    let screen = draw(&mut e, 80, 24);
    let text = (0..24).map(|y| row(&screen, y)).collect::<String>();
    assert!(text.contains("2 hits") && text.contains("Ctrl+K"));
    assert!(!row(&screen, 23).contains("FIND"));
    let y = (0..23)
        .find(|&y| {
            row(&screen, y).contains("next")
                && row(&screen, y).contains("last")
                && row(&screen, y).contains('│')
        })
        .unwrap();
    e.handle(Event::Mouse(MouseEvent {
        kind: MK::Down(B::Left),
        column: 12,
        row: y,
        modifiers: M::NONE,
    }));
    assert!(e.prompt.is_none());
    assert_eq!(e.buffer.cursors[0].range(), 20..21);
    key(&mut e, K::Esc, M::NONE);
    key(&mut e, K::Char('f'), M::NONE);
    assert_eq!(e.prompt.as_ref().unwrap().value, "猫");
    assert_eq!(e.prompt.as_ref().unwrap().selected, 1);
    for (w, h) in [(16, 5), (24, 8), (40, 12), (80, 24)] {
        draw(&mut e, w, h);
    }
}

#[test]
fn find_snippets_preserve_joined_emoji_and_combining_marks() {
    let mut e = editor("context 👩‍💻 cafe\u{301} match", "text.txt");
    e.act(Action::Find);
    e.handle(Event::Paste("match".into()));
    let screen = draw(&mut e, 80, 24);
    assert!(screen.content.iter().filter(|c| c.symbol() == "👩‍💻").count() >= 2);
    assert!(
        screen
            .content
            .iter()
            .filter(|c| c.symbol() == "e\u{301}")
            .count()
            >= 2
    );
}

#[test]
fn info_help_and_modal_colors_are_distinct_and_theme_based() {
    let mut e = editor("hello\nworld", "notes.txt");
    key(&mut e, K::Char('i'), M::NONE);
    let s = draw(&mut e, 80, 24);
    assert!(row(&s, 1).contains(" Info "));
    assert_eq!(s[(2, 1)].fg, Color::Blue);
    let text = (0..24).map(|y| row(&s, y)).collect::<String>();
    assert!(text.contains("Full file path") && text.contains("File statistics"));
    e.act(Action::Help);
    let s = draw(&mut e, 80, 24);
    assert_eq!(s[(2, 1)].fg, Color::Reset);
    let text = (0..24).map(|y| row(&s, y)).collect::<String>();
    assert!(text.contains("Ctrl+I / i") && text.contains("About"));
    assert!(!text.contains("Full file path"));
    key(&mut e, K::Char('k'), M::CONTROL);
    assert_eq!(draw(&mut e, 80, 24)[(2, 1)].fg, Color::Cyan);
    key(&mut e, K::Char('f'), M::CONTROL);
    let s = draw(&mut e, 80, 24);
    assert_eq!(s[(2, 1)].fg, Color::Yellow);
    assert_eq!(s[(0, 23)].bg, Color::Blue);
    key(&mut e, K::Char('i'), M::CONTROL);
    assert!(row(&draw(&mut e, 80, 24), 1).contains(" Info "));
    assert_eq!(e.buffer.text.to_string(), "hello\nworld");
}

#[test]
fn viewport_position_ignores_clicks_and_edit_columns_include_insertion_point() {
    let mut e = editor("hello\nworld\nlast", "text.txt");
    let before = row(&draw(&mut e, 80, 10), 9);
    e.handle(Event::Mouse(MouseEvent {
        kind: MK::Down(B::Left),
        column: 3,
        row: 1,
        modifiers: M::NONE,
    }));
    assert_eq!(row(&draw(&mut e, 80, 10), 9), before);
    e.mode = Mode::Edit;
    let s = draw(&mut e, 80, 10);
    assert!(row(&s, 9).contains("2/3:4/6"));
    assert_eq!(s[(0, 9)].bg, Color::Cyan);
    e.buffer.cursors[0] = editio::buffer::Cursor::at(11);
    assert!(row(&draw(&mut e, 80, 10), 9).contains("2/3:6/6"));
    let source = (0..30).map(|i| format!("row{i}\n")).collect::<String>();
    let mut e = editor(&source, "text.txt");
    e.wrap = false;
    draw(&mut e, 80, 10);
    key(&mut e, K::Down, M::NONE);
    draw(&mut e, 80, 10);
    // Source view navigation can select a position, but the indicator tracks the viewport.
    e.scroll = 5;
    assert!(row(&draw(&mut e, 80, 10), 9).contains("45%/31"));
}

#[test]
fn view_document_has_no_cursor_but_editable_find_field_does() {
    for wrap in [false, true] {
        let mut e = editor("hello world\nhello again", "file.txt");
        e.wrap = wrap;
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        assert!(!t.backend().cursor_visible());
        e.act(Action::Find);
        e.handle(Event::Paste("hello".into()));
        t.draw(|f| e.draw(f, f.area())).unwrap();
        assert!(t.backend().cursor_visible());
        key(&mut e, K::Enter, M::NONE);
        t.draw(|f| e.draw(f, f.area())).unwrap();
        assert!(!t.backend().cursor_visible());
        key(&mut e, K::Esc, M::NONE);
        e.act(Action::ToggleMode);
        t.draw(|f| e.draw(f, f.area())).unwrap();
        assert!(t.backend().cursor_visible());
        e.act(Action::ToggleMode);
        t.draw(|f| e.draw(f, f.area())).unwrap();
        assert!(!t.backend().cursor_visible());
    }
}

#[test]
fn transient_messages_use_toasts_and_preserve_the_status_bar() {
    let mut e = editor("hello", "file.txt");
    let before = row(&draw(&mut e, 80, 24), 23);
    e.notify("Copied to clipboard");
    let s = draw(&mut e, 80, 24);
    assert_eq!(row(&s, 23), before);
    let y = (0..23)
        .find(|&y| row(&s, y).contains("Copied to clipboard"))
        .unwrap();
    let x = row(&s, y).find("Copied").unwrap() as u16;
    e.handle(Event::Mouse(MouseEvent {
        kind: MK::Down(B::Left),
        column: x,
        row: y,
        modifiers: M::NONE,
    }));
    assert!(e.message.is_empty());
    assert_eq!(e.buffer.cursors[0].head, 0);
    e.persistent_message("Confirm quit");
    assert!(row(&draw(&mut e, 80, 24), 23).contains("Confirm quit"));
}

#[test]
fn typed_toasts_use_callout_colors_and_errors_do_not_color_later_notices() {
    use editio::ToastKind;
    let mut e = editor("hello", "file.txt");
    for (kind, color) in [
        (ToastKind::Note, Color::Blue),
        (ToastKind::Tip, Color::Green),
        (ToastKind::Important, Color::Magenta),
        (ToastKind::Warning, Color::Yellow),
        (ToastKind::Caution, Color::Red),
    ] {
        e.notify_with_kind(kind, "Notification");
        let s = draw(&mut e, 80, 24);
        let y = (0..23)
            .find(|&y| row(&s, y).contains("Notification"))
            .unwrap();
        let x = row(&s, y).find("Notification").unwrap() as u16;
        assert_eq!(s[(x, y)].fg, Color::Reset);
        assert_eq!(s[(x - 2, y - 1)].fg, color);
        assert!(row(&s, 23).contains("VIEW"));
    }
    // Exercise a real validation failure, rather than only the typed API.
    e.act(Action::SaveAs);
    key(&mut e, K::Enter, M::NONE);
    let s = draw(&mut e, 80, 24);
    let y = (0..23)
        .find(|&y| row(&s, y).contains("Filename cannot be empty"))
        .unwrap();
    let x = row(&s, y).find("Filename").unwrap() as u16;
    assert_eq!(s[(x, y)].fg, Color::Reset);
    assert_eq!(s[(x - 2, y - 1)].fg, Color::Red);
    e.notify("Normal notice");
    let s = draw(&mut e, 80, 24);
    let y = (0..23)
        .find(|&y| row(&s, y).contains("Normal notice"))
        .unwrap();
    let x = row(&s, y).find("Normal notice").unwrap() as u16;
    assert_eq!(s[(x, y)].fg, Color::Reset);
    assert_eq!(s[(x - 2, y - 1)].fg, Color::Blue);
    e.monochrome = true;
    e.notify_error("Monochrome error");
    let s = draw(&mut e, 80, 24);
    let y = (0..23)
        .find(|&y| row(&s, y).contains("Monochrome error"))
        .unwrap();
    let x = row(&s, y).find("Monochrome error").unwrap() as u16;
    assert_eq!(s[(x, y)].fg, Color::Reset);
}

#[test]
fn toast_markdown_is_opt_in_and_preserves_styles_when_wrapped() {
    use editio::ToastKind;
    let mut e = editor("hello", "file.txt");
    e.notify("**Literal** `code` [link](url)");
    let s = draw(&mut e, 80, 24);
    assert!((0..23).any(|y| row(&s, y).contains("**Literal** `code` [link](url)")));
    for width in [24, 80] {
        e.notify_markdown(
            ToastKind::Warning,
            "**Bold message** and *italic* with `code` and [link](https://hidden.example)",
        );
        let s = draw(&mut e, width, 24);
        let rows: String = (0..23).map(|y| row(&s, y)).collect();
        assert!(!rows.contains("**") && !rows.contains("https://hidden.example"));
        let y = (0..23).find(|&y| row(&s, y).contains("Bold")).unwrap();
        let x = row(&s, y).find("Bold").unwrap() as u16;
        assert!(s[(x, y)].modifier.contains(Modifier::BOLD));
        assert_eq!(s[(x, y)].fg, Color::Reset);
        assert_eq!(s[(x - 2, y - 1)].fg, Color::Yellow);
    }
    e.notify("**Plain again**");
    assert!((0..23).any(|y| row(&draw(&mut e, 80, 24), y).contains("**Plain again**")));
}

#[test]
fn viewport_arrows_scroll_after_search_exit_in_every_source_view() {
    let text = (0..60)
        .map(|i| format!("row{i} target\n"))
        .collect::<String>();
    for path in ["file.txt", "file.md", "file.csv"] {
        for wrap in [false, true] {
            let mut e = editor(&text, path);
            e.source = true;
            e.wrap = wrap;
            draw(&mut e, 80, 12);
            e.act(Action::Find);
            e.handle(Event::Paste("target".into()));
            key(&mut e, K::Enter, M::NONE);
            draw(&mut e, 80, 12);
            key(&mut e, K::Esc, M::NONE);
            let selection = e.buffer.cursors[0].range();
            let top = e.scroll;
            key(&mut e, K::Down, M::NONE);
            draw(&mut e, 80, 12);
            assert_eq!(e.scroll, top + 1, "{path} wrap={wrap}");
            key(&mut e, K::Up, M::NONE);
            draw(&mut e, 80, 12);
            assert_eq!(e.scroll, top);
            key(&mut e, K::PageDown, M::NONE);
            draw(&mut e, 80, 12);
            assert_eq!(e.scroll, top + 11);
            assert_eq!(e.buffer.cursors[0].range(), selection);
            e.act(Action::Find);
            key(&mut e, K::Esc, M::NONE);
            key(&mut e, K::Up, M::NONE);
            draw(&mut e, 80, 12);
            assert_eq!(e.scroll, top + 10);
        }
    }
}

#[test]
fn find_palette_is_scoped_and_returns_to_preserved_search() {
    use editio::PromptKind;
    let mut e = editor("cat CAT", "file.txt");
    assert!(!e.search_options.case_sensitive);
    e.act(Action::Find);
    e.handle(Event::Paste("cat".into()));
    key(&mut e, K::Down, M::NONE);
    key(&mut e, K::Char('k'), M::CONTROL);
    let commands = e.matching_commands();
    assert_eq!(commands.len(), 3);
    assert!(commands.iter().all(|c| matches!(
        c.action,
        Action::ToggleSearchCase | Action::ToggleSearchFuzzy | Action::ToggleSearchWildcards
    )));
    key(&mut e, K::Esc, M::NONE);
    assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::Find);
    assert_eq!(e.prompt.as_ref().unwrap().value, "cat");
    assert_eq!(e.prompt.as_ref().unwrap().selected, 1);
    key(&mut e, K::Char('k'), M::CONTROL);
    e.handle(Event::Paste("case".into()));
    key(&mut e, K::Enter, M::NONE);
    assert!(e.search_options.case_sensitive);
    assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::Find);
    assert_eq!(e.prompt.as_ref().unwrap().value, "cat");
    key(&mut e, K::Char('u'), M::CONTROL);
    assert_eq!(e.prompt.as_ref().unwrap().kind, PromptKind::Find);
    assert!(e.prompt.as_ref().unwrap().value.is_empty());
    key(&mut e, K::Esc, M::NONE);
    key(&mut e, K::Char('k'), M::CONTROL);
    assert!(
        e.matching_commands()
            .iter()
            .any(|c| matches!(c.action, Action::Info))
    );
}

#[test]
fn find_footer_stays_one_line_and_keeps_the_palette_hint_at_narrow_widths() {
    for width in [16, 24, 32, 48, 80] {
        let mut e = editor("a A a", "file.txt");
        e.act(Action::Find);
        e.handle(Event::Paste("a".into()));
        let s = draw(&mut e, width, 20);
        let footer = (0..19).find(|&y| row(&s, y).contains("Ctrl+K")).unwrap();
        assert!(row(&s, footer).contains('3'), "{}", row(&s, footer));
        assert!(
            row(&s, footer).trim_end().ends_with("Ctrl+K │"),
            "{}",
            row(&s, footer)
        );
        assert!(row(&s, footer + 1).contains('└'));
        assert!(!row(&s, footer).contains("Shift+Tab"));
    }
}

#[test]
fn static_modals_fit_content_and_narrow_status_prefers_filename() {
    let mut e = editor("hello", "document-name.txt");
    for action in [Action::Help, Action::Info] {
        e.act(action);
        let s = draw(&mut e, 100, 80);
        let top = (0..80).find(|&y| row(&s, y).contains('┌')).unwrap();
        let bottom = (0..80).find(|&y| row(&s, y).contains('└')).unwrap();
        assert!(bottom - top < 60);
        assert!((bottom - 2..bottom).any(|y| !row(&s, y).trim_matches(['│', ' ']).is_empty()));
        let small = draw(&mut e, 32, 8);
        assert!((0..7).any(|y| row(&small, y).contains('└')));
    }
    key(&mut e, K::Esc, M::NONE);
    for width in [32, 48] {
        let status = row(&draw(&mut e, width, 12), 11);
        assert!(!status.contains("wrap:"));
        assert!(status.contains(".txt"), "{status}");
        assert!(status.ends_with("Ctrl+?"));
        assert!(!status.ends_with("help"));
    }
}

#[test]
fn help_has_no_blank_last_content_row_when_it_fits() {
    let mut e = editor("hello", "file.txt");
    for (width, height) in [(80, 40), (81, 41), (100, 60), (101, 61)] {
        e.act(Action::Help);
        let screen = draw(&mut e, width, height);
        let bottom = (0..height)
            .find(|&y| row(&screen, y).contains('└'))
            .unwrap();
        assert!(!row(&screen, bottom - 1).trim_matches(['│', ' ']).is_empty());
    }
}

#[test]
fn find_dialog_leaves_main_bar_unchanged_until_accept_and_empty_enter_exits() {
    for mode in [Mode::View, Mode::Edit] {
        for width in [32, 80, 160] {
            let mut e = editor("cat dog cat", "find.txt");
            e.mode = mode;
            let before = draw(&mut e, width, 24);
            let baseline = row(&before, 23);
            e.act(Action::Find);
            e.handle(Event::Paste("cat".into()));
            let dialog = draw(&mut e, width, 24);
            assert_eq!(row(&dialog, 23), baseline);
            assert_eq!(dialog[(0, 23)].fg, before[(0, 23)].fg);
            assert_eq!(dialog[(0, 23)].bg, before[(0, 23)].bg);
            key(&mut e, K::Enter, M::NONE);
            let active = draw(&mut e, width, 24);
            let bar = row(&active, 23);
            assert!(bar.contains("FIND 1/2"), "{bar}");
            assert!(
                bar.contains("↑↓") && bar.contains("Esc") && bar.ends_with("Ctrl+?"),
                "{bar}"
            );
            assert!(
                !bar.contains("Tab")
                    && !bar.contains("PgUp")
                    && !bar.contains("Ctrl+F")
                    && !bar.contains("Ctrl+K")
            );
            assert_eq!(active[(0, 23)].bg, Color::Yellow);
            assert_eq!(active[(0, 23)].fg, Color::Black);
            assert!(
                !active[(0, 23)]
                    .modifier
                    .intersects(Modifier::REVERSED | Modifier::DIM)
            );
            e.act(Action::Find);
            key(&mut e, K::Char('u'), M::CONTROL);
            key(&mut e, K::Enter, M::NONE);
            assert!(e.prompt.is_none());
            let ended = draw(&mut e, width, 24);
            assert!(!row(&ended, 23).contains("FIND"));
            assert_eq!(ended[(0, 23)].fg, before[(0, 23)].fg);
            assert_eq!(ended[(0, 23)].bg, before[(0, 23)].bg);
            assert_eq!(e.mode, mode);
            e.act(Action::Find);
            assert!(e.prompt.as_ref().unwrap().value.is_empty());
            key(&mut e, K::Enter, M::NONE);
            assert!(e.prompt.is_none());
        }
    }
}

#[test]
fn view_percentage_uses_visible_end_at_every_width_and_reaches_bottom() {
    let source = (1..=30).map(|i| format!("row{i}\n")).collect::<String>();
    for path in ["file.txt", "file.md"] {
        let content = if path.ends_with(".md") {
            source.replace("\n", "  \n")
        } else {
            source.clone()
        };
        let mut e = editor(&content, path);
        e.wrap = false;
        let wide = draw(&mut e, 80, 10);
        let status = row(&wide, 9);
        assert!(
            status.contains(if path.ends_with(".md") {
                "30%/30"
            } else {
                "29%/31"
            }),
            "{path}: {status}"
        );
        let narrow = row(&draw(&mut e, 40, 10), 9);
        assert!(
            narrow.contains(if path.ends_with(".md") {
                "30%/30"
            } else {
                "29%/31"
            }),
            "{narrow}"
        );
        e.scroll = 25;
        let end = row(&draw(&mut e, 80, 10), 9);
        assert!(
            end.contains("100%/31") || end.contains("100%/30"),
            "{path}: {end}"
        );
    }
    let mut e = editor(&format!("{}\nlast", "word ".repeat(100)), "file.txt");
    let wrapped = row(&draw(&mut e, 80, 4), 3);
    assert!(wrapped.contains("37%/2"), "{wrapped}");
    e.scroll = usize::MAX / 2;
    assert!(row(&draw(&mut e, 80, 4), 3).contains("100%/2"));
}

#[test]
fn info_path_directories_are_normal_and_filename_accent_survives_wrapping() {
    for path in [
        "/directoryZZZ/subfolderZZZ/fileQQQ.md",
        r"C:\directoryZZZ\subfolderZZZ\fileQQQ.md",
    ] {
        for width in [32, 100] {
            let mut e = editor("text", path);
            e.act(Action::Info);
            let screen = draw(&mut e, width, 50);
            let mut directories = 0;
            let mut filename = 0;
            for cell in &screen.content[..screen.content.len() - width as usize] {
                if cell.symbol() == "Z" {
                    assert_eq!(cell.fg, Color::Reset);
                    directories += 1;
                }
                if cell.symbol() == "Q" {
                    assert_eq!(cell.fg, Color::Blue);
                    filename += 1;
                }
            }
            assert_eq!(directories, 6, "{path}, {width}");
            assert_eq!(filename, 3, "{path}, {width}");
        }
    }
}

#[test]
fn find_keyboard_cycles_while_mouse_wheel_stops_at_edges() {
    let mut e = editor("needle first\nneedle last", "file.txt");
    key(&mut e, K::Char('f'), M::NONE);
    e.handle(Event::Paste("needle".into()));
    let screen = draw(&mut e, 80, 24);
    let y = (0..23)
        .find(|&y| row(&screen, y).contains("needle first"))
        .unwrap();
    let wheel = |kind| {
        Event::Mouse(MouseEvent {
            kind,
            column: 12,
            row: y,
            modifiers: M::NONE,
        })
    };
    e.handle(wheel(MK::ScrollUp));
    assert_eq!(e.prompt.as_ref().unwrap().selected, 0);
    key(&mut e, K::Up, M::NONE);
    assert_eq!(e.prompt.as_ref().unwrap().selected, 1);
    e.handle(wheel(MK::ScrollDown));
    assert_eq!(e.prompt.as_ref().unwrap().selected, 1);
    key(&mut e, K::Down, M::NONE);
    assert_eq!(e.prompt.as_ref().unwrap().selected, 0);
}
