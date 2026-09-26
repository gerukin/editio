//! Modal decisions for an external disk update; the host owns all effects.
use crossterm::event::{Event, KeyCode as K, KeyEventKind, MouseButton, MouseEventKind};
use editio::{Editor, buffer::Buffer};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use tapp_ui::{modal, theme::Role};

pub enum Choice {
    Copy,
    Compare,
    Disk,
}
pub struct Conflict {
    pub disk: Buffer,
    selected: usize,
    hits: Vec<(usize, Rect)>,
}
impl Conflict {
    pub fn new(disk: Buffer) -> Self {
        Self {
            disk,
            selected: 2,
            hits: vec![],
        }
    }
    pub fn draw(&mut self, f: &mut Frame, editor: &Editor) {
        let labels = [
            "Discard edits / load disk",
            "Copy + compare",
            "Keep unsaved copy",
        ];
        let block = modal::block(
            " File changed on disk ",
            " Esc: keep copy ",
            modal::Category::Confirmation,
            editor.monochrome,
        );
        let bounds = modal::content_rect(f.area(), 86, usize::MAX);
        let width = block.inner(bounds).width.max(1);
        let rows = modal::button_rows(labels, width);
        let path = editor
            .buffer
            .path
            .as_deref()
            .map(tapp_ui::path::display)
            .unwrap_or_default();
        let paragraph = Paragraph::new(vec![
            Line::from("The file changed outside Editio. Your unsaved edits are still here."),
            Line::from(tapp_ui::path::PathDisplay::new(&path, editor.monochrome).spans()),
            Line::from(""),
            Line::from("Keep a copy in memory, optionally comparing with the new disk version, or discard your edits and reload."),
        ]).wrap(Wrap { trim: false });
        let area = modal::content_rect(f.area(), 86, paragraph.line_count(width) + rows + 1);
        let inner = modal::draw_frame(f, area, block);
        f.render_widget(
            paragraph,
            Rect::new(
                inner.x,
                inner.y,
                inner.width,
                inner.height.saturating_sub(rows as u16 + 1),
            ),
        );
        self.hits = modal::buttons(
            f,
            inner,
            &[
                modal::Button {
                    label: labels[0],
                    role: Role::Danger,
                },
                modal::Button {
                    label: labels[1],
                    role: Role::Text,
                },
                modal::Button {
                    label: labels[2],
                    role: Role::Command,
                },
            ],
            self.selected,
            editor.monochrome,
        );
        if !editor.monochrome {
            editor.theme.apply(f.buffer_mut(), area);
        }
    }
    pub fn handle(&mut self, event: &Event) -> Option<Choice> {
        let selected = match event {
            Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                K::Esc => Some(2),
                K::Enter => Some(self.selected),
                K::Up | K::Left | K::BackTab => {
                    self.selected = (self.selected + 2) % 3;
                    None
                }
                K::Down | K::Right | K::Tab => {
                    self.selected = (self.selected + 1) % 3;
                    None
                }
                _ => None,
            },
            Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => self
                .hits
                .iter()
                .find_map(|(i, r)| r.contains((m.column, m.row).into()).then_some(*i)),
            _ => None,
        };
        selected.map(|i| match i {
            0 => Choice::Disk,
            1 => Choice::Compare,
            _ => Choice::Copy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};
    #[test]
    fn conflict_is_content_sized_cursorless_and_safe_by_default() {
        for width in [20, 40, 100] {
            let e = Editor::new(Buffer::new("unsaved"));
            let mut dialog = Conflict::new(Buffer::new("external"));
            let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
            terminal.draw(|f| dialog.draw(f, &e)).unwrap();
            assert!(!terminal.backend().cursor_visible());
            assert!(matches!(
                dialog.handle(&Event::Key(KeyEvent::new(K::Enter, KeyModifiers::NONE))),
                Some(Choice::Copy)
            ));
            assert!(matches!(
                dialog.handle(&Event::Key(KeyEvent::new(K::Esc, KeyModifiers::NONE))),
                Some(Choice::Copy)
            ));
            assert!(
                dialog
                    .handle(&Event::Paste("must not edit".into()))
                    .is_none()
            );
            assert_eq!(e.buffer.text.to_string(), "unsaved");
        }
    }
}
