use crate::escape_sequence::{move_cursor, ESCAPE_SEQUENCE_CLEAR_LINE};
use crate::ui::{Component, Drawable};
use std::io::Error;
use std::time::SystemTime;

#[derive(Debug, PartialEq)]
pub struct MessageBar {
    component: Component,
    message: String,
    updated_at: SystemTime,
}

impl MessageBar {
    pub fn new(message: String, time: SystemTime) -> MessageBar {
        MessageBar {
            component: Component::default(),
            message,
            updated_at: time,
        }
    }

    pub fn set(&mut self, message: String, time: SystemTime) {
        self.message = message;
        self.updated_at = time;
    }

    pub fn get_message(&self, now: SystemTime) -> Option<String> {
        now.duration_since(self.updated_at)
            .map(|d| d.as_secs() < 5)
            .map_or(None, |b| if b { Some(self.message.clone()) } else { None })
    }

    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.component.set_size(x, y, width, height);
    }
}

impl Drawable for MessageBar {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        let cursor = move_cursor(self.component.x(), self.component.y());
        buf.push_str(&cursor);

        buf.push_str(ESCAPE_SEQUENCE_CLEAR_LINE);

        let now = SystemTime::now();
        if let Some(message) = self.get_message(now) {
            buf.push_str(&message);
        }

        Ok(())
    }
}
