use crate::escape_sequence::{
    move_terminal_cursor, ESCAPE_SEQUENCE_STYLE_RESET, ESCAPE_SEQUENCE_STYLE_REVERSE,
};
use crate::screen::Screen;
use crate::ui::{Component, Drawable};
use std::io::Error;

#[derive(Debug, PartialEq)]
pub struct StatusBar {
    component: Component,
    left_status: String,
    right_status: String,
}

impl StatusBar {
    pub fn new() -> StatusBar {
        StatusBar {
            component: Component::default(),
            left_status: "".to_string(),
            right_status: "".to_string(),
        }
    }

    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.component.set_size(x, y, width, height);
    }

    pub fn set_left_status(&mut self, screen: &mut Screen) {
        self.left_status = format!(
            "{:<20} - {} lines {}",
            screen
                .buffer()
                .get_filepath()
                .unwrap_or_else(|| "[No Name]".to_string()),
            self.component.height(),
            if screen.buffer().is_dirty() {
                "(modified)"
            } else {
                ""
            }
        );
    }

    pub fn set_right_status(&mut self, screen: &mut Screen) {
        self.right_status = format!(
            "{} | {}/{}",
            screen
                .buffer()
                .get_file_type()
                .map_or("no ft", |ft| ft.to_str()),
            screen.get_cy() + 1,
            screen.buffer().len()
        );
    }
}

impl Drawable for StatusBar {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        let cursor = move_terminal_cursor(self.component.x(), self.component.y());
        buf.push_str(&cursor);

        buf.push_str(ESCAPE_SEQUENCE_STYLE_REVERSE);

        if self.component.width() < self.left_status.len() {
            let s: String = self
                .left_status
                .chars()
                .take(self.component.width())
                .collect();
            buf.push_str(&s);
        } else {
            buf.push_str(&self.left_status);

            if self.component.width() as isize
                - self.left_status.len() as isize
                - self.right_status.len() as isize
                > 0
            {
                for _ in
                    0..(self.component.width() - self.left_status.len() - self.right_status.len())
                {
                    buf.push(' ');
                }
                buf.push_str(&self.right_status);
            } else {
                for _ in 0..(self.component.width() - self.left_status.len()) {
                    buf.push(' ');
                }
            }
        }

        buf.push_str(ESCAPE_SEQUENCE_STYLE_RESET);
        buf.push_str("\r\n");

        Ok(())
    }
}
