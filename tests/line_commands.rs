use crossterm::event::{Event, KeyCode as K, KeyEvent, KeyModifiers as M};
use editio::{
    Action, Editor, Mode, Outcome,
    buffer::{Buffer, Cursor},
};
fn editor(text: &str, cursors: Vec<Cursor>) -> Editor {
    let mut e = Editor::new(Buffer::new(text));
    e.mode = Mode::Edit;
    e.buffer.cursors = cursors;
    e
}
fn key(e: &mut Editor, code: K, mods: M) -> Outcome {
    e.handle(Event::Key(KeyEvent::new(code, mods)))
}
fn copy_mods() -> M {
    if cfg!(target_os = "linux") {
        M::CONTROL | M::SHIFT | M::ALT
    } else {
        M::SHIFT | M::ALT
    }
}
fn cursor_mods() -> M {
    if cfg!(target_os = "linux") {
        M::SHIFT | M::ALT
    } else if cfg!(target_os = "macos") {
        M::SUPER | M::ALT
    } else {
        M::CONTROL | M::ALT
    }
}

#[test]
fn copy_line_up_down_preserves_column_and_undo_redo() {
    for (keycode, expected) in [(K::Up, 1), (K::Down, 5)] {
        let mut e = editor("猫ab\nend", vec![Cursor::at(1)]);
        assert_eq!(key(&mut e, keycode, copy_mods()), Outcome::Changed);
        assert_eq!(e.buffer.text.to_string(), "猫ab\n猫ab\nend");
        assert_eq!(e.buffer.cursors, vec![Cursor::at(expected)]);
        e.act(Action::Undo);
        assert_eq!(e.buffer.text.to_string(), "猫ab\nend");
        assert_eq!(e.buffer.cursors, vec![Cursor::at(1)]);
        e.act(Action::Redo);
        assert_eq!(e.buffer.cursors, vec![Cursor::at(expected)]);
    }
}
#[test]
fn copy_selected_block_preserves_reverse_selection_excludes_end_line() {
    for (action, shift) in [(Action::DuplicateUp, 0), (Action::DuplicateDown, 4)] {
        let original = Cursor { anchor: 4, head: 1 };
        let mut e = editor("a\nb\nc\n", vec![original.clone()]);
        e.act(action);
        assert_eq!(e.buffer.text.to_string(), "a\nb\na\nb\nc\n");
        assert_eq!(
            e.buffer.cursors,
            vec![Cursor {
                anchor: 4 + shift,
                head: 1 + shift
            }]
        );
        e.act(Action::Undo);
        assert_eq!(e.buffer.cursors, vec![original]);
    }
}
#[test]
fn copy_final_line_and_empty_lines_preserves_newline_shape() {
    for (text, pos, expected) in [
        ("a\nlast", 4, "a\nlast\nlast"),
        ("a\n", 2, "a\n\n"),
        ("", 0, "\n"),
    ] {
        for action in [Action::DuplicateUp, Action::DuplicateDown] {
            let mut e = editor(text, vec![Cursor::at(pos)]);
            e.act(action);
            assert_eq!(e.buffer.text.to_string(), expected);
            assert!(e.buffer.cursors[0].head <= e.buffer.text.len_chars());
            e.act(Action::Undo);
            assert_eq!(e.buffer.text.to_string(), text);
        }
    }
}
#[test]
fn duplicate_all_cursor_lines_once_and_adjust_positions() {
    let original = vec![Cursor::at(1), Cursor::at(2), Cursor::at(9)];
    let mut e = editor("abc\nmid\nxyz", original.clone());
    e.act(Action::DuplicateDown);
    assert_eq!(e.buffer.text.to_string(), "abc\nabc\nmid\nxyz\nxyz");
    assert_eq!(
        e.buffer.cursors,
        vec![Cursor::at(5), Cursor::at(6), Cursor::at(17)]
    );
    e.act(Action::Undo);
    assert_eq!(e.buffer.cursors, original);
    let mut e = editor("a\nb", vec![Cursor::at(0), Cursor::at(2)]);
    e.act(Action::DuplicateDown);
    assert_eq!(e.buffer.text.to_string(), "a\na\nb\nb");
    assert_eq!(e.buffer.cursors, vec![Cursor::at(2), Cursor::at(6)]);
}
#[test]
fn delete_current_and_selected_lines_not_only_selection() {
    let mut e = editor("one\ntwo\nthree\nfour", vec![Cursor { anchor: 5, head: 9 }]);
    assert_eq!(
        key(&mut e, K::Char('K'), M::CONTROL | M::SHIFT),
        Outcome::Changed
    );
    assert_eq!(e.buffer.text.to_string(), "one\nfour");
    assert_eq!(e.buffer.cursors, vec![Cursor::at(5)]);
    e.act(Action::Undo);
    assert_eq!(e.buffer.text.to_string(), "one\ntwo\nthree\nfour");
    assert_eq!(e.buffer.cursors, vec![Cursor { anchor: 5, head: 9 }]);
    let mut e = editor("a\nb\nc", vec![Cursor { anchor: 2, head: 4 }]);
    e.act(Action::DeleteLine);
    assert_eq!(e.buffer.text.to_string(), "a\nc");
}
#[test]
fn delete_last_line_consumes_separator_and_keeps_column() {
    for (text, pos, expected, cursor) in [
        ("abcd\nxy", 6, "abcd", 1),
        ("a\n", 2, "a", 0),
        ("only", 2, "", 0),
        ("", 0, "", 0),
    ] {
        let mut e = editor(text, vec![Cursor::at(pos)]);
        e.act(Action::DeleteLine);
        assert_eq!(e.buffer.text.to_string(), expected);
        assert_eq!(e.buffer.cursors, vec![Cursor::at(cursor)]);
        if text.is_empty() {
            assert!(!e.buffer.dirty());
        } else {
            e.act(Action::Undo);
            assert_eq!(e.buffer.text.to_string(), text);
        }
    }
}
#[test]
fn delete_overlapping_adjacent_and_disjoint_cursor_lines() {
    let mut e = editor(
        "aa\nbb\ncc\ndd",
        vec![Cursor::at(1), Cursor::at(2), Cursor::at(10)],
    );
    e.act(Action::DeleteLine);
    assert_eq!(e.buffer.text.to_string(), "bb\ncc");
    assert!(e.buffer.cursors.iter().all(|c| c.head <= 5));
    e.act(Action::Undo);
    assert_eq!(e.buffer.text.to_string(), "aa\nbb\ncc\ndd");
    let mut e = editor("a\nb\nc", vec![Cursor::at(2), Cursor::at(4)]);
    e.act(Action::DeleteLine);
    assert_eq!(e.buffer.text.to_string(), "a");
    let mut e = editor(
        "a\nb\nc",
        vec![Cursor { anchor: 0, head: 4 }, Cursor { anchor: 2, head: 5 }],
    );
    e.act(Action::DeleteLine);
    assert_eq!(e.buffer.text.to_string(), "");
    assert_eq!(e.buffer.cursors, vec![Cursor::at(0)]);
}
#[test]
fn repeat_add_above_and_below_then_edit_every_cursor() {
    for (code, pos) in [(K::Up, 9), (K::Down, 1)] {
        let mut e = editor("abc\ndef\nghi", vec![Cursor::at(pos)]);
        key(&mut e, code, cursor_mods());
        key(&mut e, code, cursor_mods());
        key(&mut e, code, cursor_mods());
        assert_eq!(e.buffer.cursors.len(), 3);
        key(&mut e, K::Char('X'), M::NONE);
        assert_eq!(e.buffer.text.to_string(), "aXbc\ndXef\ngXhi");
        e.act(Action::Undo);
        assert_eq!(e.buffer.text.to_string(), "abc\ndef\nghi");
        assert_eq!(e.buffer.cursors.len(), 3);
    }
}
#[test]
fn cursor_add_preserves_visual_column_across_short_lines_and_unicode() {
    let mut e = editor("abcd\nx\n猫👩‍💻z\n\tab", vec![Cursor::at(4)]);
    e.override_indentation(editio::Indentation {
        tabs: true,
        size: 4,
        tab_width: 4,
    });
    for _ in 0..3 {
        key(&mut e, K::Down, M::CONTROL | M::ALT);
    }
    let mut heads: Vec<_> = e.buffer.cursors.iter().map(|c| c.head).collect();
    heads.sort();
    // Column 4: clamp on x, after whole emoji on the wide line, after the tab.
    assert_eq!(heads, vec![4, 6, 11, 14]);
}
#[test]
fn line_shortcuts_respect_modes_modals_and_host_handoff() {
    let mut e = editor("abc", vec![Cursor::at(1)]);
    e.mode = Mode::View;
    key(&mut e, K::Down, copy_mods());
    key(&mut e, K::Char('k'), M::CONTROL | M::SHIFT);
    assert_eq!(e.buffer.text.to_string(), "abc");
    e.mode = Mode::Edit;
    key(&mut e, K::F(2), M::NONE);
    key(&mut e, K::Down, copy_mods());
    assert_eq!(e.buffer.text.to_string(), "abc");
    key(&mut e, K::Esc, M::NONE);
    e.handoff_keys.push(KeyEvent::new(K::Down, copy_mods()));
    assert!(matches!(
        key(&mut e, K::Down, copy_mods()),
        Outcome::FocusReleased(_)
    ));
    assert_eq!(e.buffer.text.to_string(), "abc");
    assert_eq!(key(&mut e, K::Char('k'), M::CONTROL), Outcome::Handled);
    assert!(e.prompt.is_some());
    assert_eq!(e.buffer.text.to_string(), "abc");
}

#[test]
fn secondary_cursors_remain_visible_at_line_ends_and_on_blank_lines() {
    use ratatui::{Terminal, backend::TestBackend, style::Modifier};
    for (text, cursors, x) in [
        ("a\n", vec![Cursor::at(2), Cursor::at(1)], 1),
        ("\na", vec![Cursor::at(2), Cursor::at(0)], 0),
    ] {
        let mut e = editor(text, cursors);
        let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
        terminal.draw(|frame| e.draw(frame, frame.area())).unwrap();
        let cell = &terminal.backend().buffer()[(x, 0)];
        assert!(
            cell.modifier
                .contains(Modifier::REVERSED | Modifier::UNDERLINED)
        );
    }
}

#[test]
fn ctrl_u_removes_only_logical_prefixes_and_undo_restores_cursors() {
    let source = "猫abc\r\nsecond\r\nlast";
    let cursors = vec![Cursor::at(2), Cursor::at(3), Cursor::at(8), Cursor::at(14)];
    let mut e = editor(source, cursors.clone());
    key(&mut e, K::Char('u'), M::CONTROL);
    assert_eq!(e.buffer.text.to_string(), "c\r\ncond\r\nlast");
    e.act(Action::Undo);
    assert_eq!(e.buffer.text.to_string(), source);
    assert_eq!(e.buffer.cursors, cursors);
    e.act(Action::Redo);
    assert_eq!(e.buffer.text.to_string(), "c\r\ncond\r\nlast");
    let revision = e.buffer.revision();
    key(&mut e, K::Char('u'), M::CONTROL);
    assert_eq!(e.buffer.revision(), revision);
}

#[test]
fn down_at_final_display_row_reaches_end_with_selection_and_wrapping() {
    for wrap in [false, true] {
        for navigation in [K::Down, K::PageDown] {
            let source = "first\nlast words with a long final line";
            let end = source.chars().count();
            let mut e = editor(source, vec![Cursor::at(end - 2)]);
            e.wrap = wrap;
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(16, 12)).unwrap();
            terminal.draw(|f| e.draw(f, f.area())).unwrap();
            key(&mut e, navigation, M::SHIFT);
            assert_eq!(
                e.buffer.cursors[0],
                Cursor {
                    anchor: end - 2,
                    head: end
                }
            );
            key(&mut e, navigation, M::NONE);
            assert_eq!(e.buffer.cursors[0], Cursor::at(end));
            assert_eq!(e.buffer.text.to_string(), source);
        }
    }
}

#[test]
fn home_end_use_document_bounds_with_unicode_selection_and_multiple_cursors() {
    for wrap in [false, true] {
        for modifier in [M::CONTROL] {
            let source = "猫abc\r\nsecond line\r\nlast";
            let original = vec![Cursor::at(2), Cursor::at(9)];
            let mut e = editor(source, original.clone());
            e.wrap = wrap;
            let end = source.chars().count();
            key(&mut e, K::End, modifier | M::SHIFT);
            for (cursor, before) in e.buffer.cursors.iter().zip(&original) {
                assert_eq!(cursor.anchor, before.head);
                assert_eq!(cursor.head, end);
            }
            key(&mut e, K::Home, modifier);
            assert!(e.buffer.cursors.iter().all(|c| *c == Cursor::at(0)));
            key(&mut e, K::End, modifier);
            assert!(e.buffer.cursors.iter().all(|c| *c == Cursor::at(end)));
            assert_eq!(e.buffer.text.to_string(), source);
        }
    }
}

#[test]
fn home_end_work_without_preparing_a_large_wrapped_document() {
    let source = "long source line with unicode 猫 and words\n".repeat(150_000);
    let mut e = editor(&source, vec![Cursor::at(1)]);
    e.wrap = true;
    key(&mut e, K::End, M::CONTROL);
    assert_eq!(e.buffer.cursors[0].head, source.chars().count());
    key(&mut e, K::Home, M::CONTROL);
    assert_eq!(e.buffer.cursors[0], Cursor::at(0));
    let mut empty = editor("", vec![Cursor::at(0)]);
    key(&mut empty, K::End, M::NONE);
    key(&mut empty, K::Home, M::SHIFT);
    assert_eq!(empty.buffer.cursors[0], Cursor::at(0));
}

#[test]
fn alt_arrows_move_lines_and_undo_without_hijacking_modal_input() {
    for (code, expected) in [(K::Up, "two\none\nthree"), (K::Down, "one\nthree\ntwo")] {
        let mut e = editor("one\ntwo\nthree", vec![Cursor::at(5)]);
        assert_eq!(key(&mut e, code, M::ALT), Outcome::Changed);
        assert_eq!(e.buffer.text.to_string(), expected);
        e.act(Action::Undo);
        assert_eq!(e.buffer.text.to_string(), "one\ntwo\nthree");
        assert_eq!(e.buffer.cursors, vec![Cursor::at(5)]);
        e.act(Action::Find);
        key(&mut e, code, M::ALT);
        assert_eq!(e.buffer.text.to_string(), "one\ntwo\nthree");
    }
    let mut e = editor("one\ntwo", vec![Cursor::at(0)]);
    assert_eq!(key(&mut e, K::Up, M::ALT), Outcome::Handled);
    assert_eq!(e.buffer.text.to_string(), "one\ntwo");
    e.command_scope
        .set_command_enabled(Action::MoveDown as u64, false);
    assert_eq!(key(&mut e, K::Down, M::ALT), Outcome::Unhandled);
    assert_eq!(e.buffer.text.to_string(), "one\ntwo");
}

#[test]
fn move_selected_blocks_preserves_direction_columns_endings_and_undo() {
    for ending in ["\n", "\r\n", "\u{2028}"] {
        let source = format!("top{ending}猫abc{ending}middle{ending}last");
        for down in [false, true] {
            for reverse in [false, true] {
                let start = 3 + ending.chars().count();
                let end = start + 4 + ending.chars().count() + 6 + ending.chars().count();
                let cursor = if reverse {
                    Cursor {
                        anchor: end,
                        head: start + 1,
                    }
                } else {
                    Cursor {
                        anchor: start + 1,
                        head: end,
                    }
                };
                let mut e = editor(&source, vec![cursor.clone()]);
                key(&mut e, if down { K::Down } else { K::Up }, M::ALT);
                let expected = if down {
                    format!("top{ending}last{ending}猫abc{ending}middle")
                } else {
                    format!("猫abc{ending}middle{ending}top{ending}last")
                };
                assert_eq!(e.buffer.text.to_string(), expected);
                assert_eq!(
                    e.buffer.cursors[0].head < e.buffer.cursors[0].anchor,
                    reverse
                );
                let selected = e.buffer.selected();
                assert!(selected.starts_with("abc"));
                assert!(selected.trim_end().ends_with("middle"));
                e.act(Action::Undo);
                assert_eq!(e.buffer.text.to_string(), source);
                assert_eq!(e.buffer.cursors, vec![cursor]);
            }
        }
    }
}

#[test]
fn palette_alias_and_shift_click_preserve_context() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for code in [K::Char('p'), K::Char('P')] {
        for mode in [Mode::Edit, Mode::View] {
            let mut e = editor("abc\ndef", vec![Cursor::at(1)]);
            e.mode = mode;
            key(&mut e, code, M::CONTROL | M::SHIFT);
            assert_eq!(e.prompt.as_ref().unwrap().kind, editio::PromptKind::Palette);
        }
    }
    let mut e = editor("猫abc\nsecond", vec![Cursor::at(1)]);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(30, 8)).unwrap();
    terminal.draw(|f| e.draw(f, f.area())).unwrap();
    for column in [4, 2] {
        e.handle(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row: 1,
            modifiers: M::SHIFT,
        }));
        assert_eq!(
            e.buffer.cursors[0],
            Cursor {
                anchor: 1,
                head: 5 + column as usize
            }
        );
    }
}

#[test]
fn home_end_target_each_current_line_and_shift_preserves_anchor() {
    let mut e = editor(
        "猫abc\r\nsecond line\r\nlast",
        vec![Cursor::at(2), Cursor::at(9)],
    );
    e.wrap = false;
    key(&mut e, K::End, M::SHIFT);
    assert_eq!(
        e.buffer.cursors,
        vec![
            Cursor { anchor: 2, head: 4 },
            Cursor {
                anchor: 9,
                head: 17
            }
        ]
    );
    key(&mut e, K::Home, M::NONE);
    assert_eq!(e.buffer.cursors, vec![Cursor::at(0), Cursor::at(6)]);
}

#[test]
fn moving_one_selected_character_always_moves_its_entire_line() {
    for reverse in [false, true] {
        for (keycode, expected, shift) in [
            (K::Up, "middle\nfirst\nlast", -6isize),
            (K::Down, "first\nlast\nmiddle", 5),
        ] {
            let cursor = if reverse {
                Cursor { anchor: 9, head: 8 }
            } else {
                Cursor { anchor: 8, head: 9 }
            };
            let mut e = editor("first\nmiddle\nlast", vec![cursor.clone()]);
            assert_eq!(key(&mut e, keycode, M::ALT), Outcome::Changed);
            assert_eq!(e.buffer.text.to_string(), expected);
            assert_eq!(
                e.buffer.cursors,
                vec![Cursor {
                    anchor: cursor.anchor.checked_add_signed(shift).unwrap(),
                    head: cursor.head.checked_add_signed(shift).unwrap()
                }]
            );
            e.act(Action::Undo);
            assert_eq!(e.buffer.text.to_string(), "first\nmiddle\nlast");
            assert_eq!(e.buffer.cursors, vec![cursor]);
        }
    }
}
