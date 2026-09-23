use crossterm::event::{Event, KeyCode as K, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use tapp_ui::{modal, theme::Role};
pub enum Choice {
    Recover,
    Disk,
    Quit,
}
pub struct Prompt {
    label: String,
    selected: usize,
    buttons: Vec<(usize, Rect)>,
}
impl Prompt {
    pub fn new(label: String) -> Self {
        Self {
            label,
            selected: 1,
            buttons: vec![],
        }
    }
    pub fn draw(&mut self, f: &mut Frame, monochrome: bool, theme: tapp_ui::theme::Theme) {
        let block = modal::block(
            " Recovery draft found ",
            " Esc: leave ",
            modal::Category::Confirmation,
            monochrome,
        );
        let labels = ["Disk: discard draft", "Recover draft"];
        let bounds = modal::content_rect(f.area(), 90, usize::MAX);
        let width = block.inner(bounds).width.max(1);
        let rows = modal::button_rows(labels.iter(), width) as u16;
        let paragraph = Paragraph::new(vec![
            Line::from("A saved recovery draft is available."),
            Line::from(tapp_ui::path::PathDisplay::new(&self.label, monochrome).spans()),
            Line::from(""),
            Line::from("Recover compares the draft with the current disk version. Save accepts it and ends comparison."),
            Line::from("Disk opens the file as it is now and permanently discards this draft."),
        ]).wrap(Wrap { trim: false });
        let area = modal::content_rect(
            f.area(),
            90,
            paragraph.line_count(width) + rows as usize + 1,
        );
        let inner = modal::draw_frame(f, area, block);
        let content = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            inner.height.saturating_sub(rows + 1),
        );
        f.render_widget(paragraph, content);
        self.buttons = modal::buttons(
            f,
            inner,
            &[
                modal::Button {
                    label: labels[0],
                    role: Role::Danger,
                },
                modal::Button {
                    label: labels[1],
                    role: Role::Command,
                },
            ],
            self.selected,
            monochrome,
        );
        if !monochrome {
            theme.apply(f.buffer_mut(), area);
        }
    }
    pub fn handle(&mut self, event: &Event) -> Option<Choice> {
        let selected = match event {
            Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                K::Esc => return Some(Choice::Quit),
                K::Left | K::Right | K::Up | K::Down | K::Tab | K::BackTab => {
                    self.selected = 1 - self.selected;
                    None
                }
                K::Enter => Some(self.selected),
                _ => None,
            },
            Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => self
                .buttons
                .iter()
                .find_map(|(i, r)| r.contains((m.column, m.row).into()).then_some(*i)),
            _ => None,
        };
        selected.map(|i| {
            if i == 0 {
                Choice::Disk
            } else {
                Choice::Recover
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_prompt_keeps_safe_default_and_escape_without_a_cursor() {
        use crossterm::event::{KeyEvent, KeyModifiers};
        use ratatui::{Terminal, backend::TestBackend};
        for width in [20, 40, 100] {
            let mut prompt = Prompt::new("/directoryZZZ/subfolderZZZ/fileQQQ.md".into());
            let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
            terminal
                .draw(|f| prompt.draw(f, false, tapp_ui::theme::Theme::Terminal))
                .unwrap();
            assert!(!terminal.backend().cursor_visible());
            let buffer = terminal.backend().buffer();
            let mut filename_chars = 0;
            for cell in &buffer.content {
                if cell.symbol() == "Z" {
                    assert_eq!(cell.fg, Color::Reset);
                }
                if cell.symbol() == "Q" {
                    filename_chars += 1;
                    assert_eq!(cell.fg, Color::Blue);
                }
            }
            assert_eq!(filename_chars, 3);
            assert!(matches!(
                prompt.handle(&Event::Key(KeyEvent::new(K::Enter, KeyModifiers::NONE))),
                Some(Choice::Recover)
            ));
            assert!(matches!(
                prompt.handle(&Event::Key(KeyEvent::new(K::Esc, KeyModifiers::NONE))),
                Some(Choice::Quit)
            ));
        }
    }
}
