//! Blocking terminal input and explicit wakeups; no periodic idle polling.
use crossterm::event::{self, Event};
use std::{io, sync::mpsc, time::Duration};

enum Message {
    Input(io::Result<Event>),
    Wake,
}
#[derive(Clone)]
pub struct Wake(mpsc::SyncSender<Message>);
impl Wake {
    pub fn notify(&self) {
        // A full queue already guarantees the main loop will wake. Never block
        // recovery completion or a console-close callback behind user input.
        let _ = self.0.try_send(Message::Wake);
    }
}
pub struct Events(mpsc::Receiver<Message>);
impl Events {
    pub fn start() -> io::Result<(Self, Wake)> {
        let (tx, rx) = mpsc::sync_channel(32);
        let wake = Wake(tx.clone());
        std::thread::Builder::new()
            .name("editio-input".into())
            .stack_size(256 * 1024)
            .spawn(move || {
                loop {
                    let event = event::read();
                    let closed = event.is_err();
                    if closed {
                        crate::shutdown::arm_exit();
                    }
                    if tx.send(Message::Input(event)).is_err() || closed {
                        break;
                    }
                }
            })?;
        Ok((Self(rx), wake))
    }
    pub fn wait(&self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        let message = match timeout {
            Some(timeout) => match self.0.recv_timeout(timeout) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => return Ok(None),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::other("Input worker stopped"));
                }
            },
            None => self
                .0
                .recv()
                .map_err(|_| io::Error::other("Input worker stopped"))?,
        };
        match message {
            Message::Input(event) => event.map(Some),
            Message::Wake => Ok(None),
        }
    }
}

#[cfg(test)]
pub fn test_wake() -> Wake {
    let (tx, _rx) = mpsc::sync_channel(1);
    Wake(tx)
}
