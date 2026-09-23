//! Bounded background stdin import. Crossterm reads keys from the controlling
//! terminal (/dev/tty on Unix, CONIN$ on Windows), independently of stdin.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use editio::{buffer::Buffer, terminal::Session};
use std::{
    io::{self, Read},
    sync::mpsc,
    time::Duration,
};

const MAX_BYTES: usize = 64 * 1024 * 1024;

fn import(reader: impl Read) -> io::Result<Buffer> {
    let mut bytes = Vec::new();
    reader.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(io::Error::other(
            "Piped input exceeds 64 MiB; redirect it to a file and open that file instead",
        ));
    }
    Buffer::from_utf8(&bytes)
}

pub fn read(session: &mut Session) -> io::Result<Buffer> {
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("stdin-import".into())
        .spawn(move || {
            let result = import(io::stdin().lock());
            let _ = send.send(result);
        })?;
    session.terminal.draw(|frame| {
        frame.render_widget(
            ratatui::widgets::Paragraph::new("Reading piped input… Esc to cancel"),
            frame.area(),
        );
    })?;
    loop {
        match receive.try_recv() {
            Ok(result) => return result,
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(io::Error::other("Stdin reader stopped unexpectedly"));
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if event::poll(Duration::from_millis(30))? {
            match event::read()? {
                Event::Key(key)
                    if key.kind != KeyEventKind::Release
                        && (key.code == KeyCode::Esc
                            || key.modifiers.contains(KeyModifiers::CONTROL)
                                && matches!(key.code, KeyCode::Char('c' | 'q'))) =>
                {
                    // The blocked reader is detached; standalone process exit closes it.
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "Piped input cancelled",
                    ));
                }
                Event::Resize(_, _) => {
                    session.terminal.draw(|frame| {
                        frame.render_widget(
                            ratatui::widgets::Paragraph::new("Reading piped input… Esc to cancel"),
                            frame.area(),
                        );
                    })?;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_import_and_preserves_file_encoding_metadata() {
        let b = import("\u{feff}hello\r\nworld\r\n".as_bytes()).unwrap();
        assert!(b.path.is_none() && b.bom && b.crlf && !b.dirty());
        assert_eq!(b.text.to_string(), "hello\nworld\n");
        for bytes in [&b"\xff"[..], &b"a\0b"[..], &b"a\rb"[..], &b"a\r\nb\n"[..]] {
            assert!(import(bytes).is_err());
        }
        assert!(import(io::repeat(b'a').take(MAX_BYTES as u64 + 1)).is_err());
    }
}
