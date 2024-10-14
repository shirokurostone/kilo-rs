use crate::buffer::EditorBuffer;
use crate::KILO_VERSION;
use std::io::{stdout, Error, Write};
use std::time::SystemTime;

trait Drawable {
    fn draw(&self, buf: &mut String) -> Result<(), Error>;
}

#[derive(Debug, PartialEq)]
pub struct EditorScreen {
    cx: usize,
    cy: usize,
    rx: usize,
    offset_x: usize,
    offset_y: usize,
    width: usize,
    height: usize,
}

impl EditorScreen {
    pub fn new() -> EditorScreen {
        EditorScreen {
            cx: 0,
            cy: 0,
            rx: 0,
            offset_x: 0,
            offset_y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cx, self.cy)
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cx = x;
        self.cy = y;
    }

    pub fn offset(&self) -> (usize, usize) {
        (self.offset_x, self.offset_y)
    }

    pub fn set_offset(&mut self, x: usize, y: usize) {
        self.offset_x = x;
        self.offset_y = y;
    }

    pub fn init_screen_size(&mut self) -> Result<(), Error> {
        let size = crossterm::terminal::size()?;

        self.width = size.0 as usize;
        self.height = size.1 as usize - 2;

        Ok(())
    }

    pub fn down(&mut self, buffer: &EditorBuffer) {
        if !buffer.is_empty() && self.cy < buffer.len() {
            self.cy += 1;
        }
    }

    pub fn up(&mut self, _: &EditorBuffer) {
        if self.cy > 0 {
            self.cy -= 1;
        }
    }

    pub fn left(&mut self, buffer: &EditorBuffer) {
        if self.cx > 0 {
            self.cx -= 1;
        } else if self.cy > 0 {
            if let Some(line) = buffer.get_line(self.cy - 1) {
                self.cy -= 1;
                self.cx = line.len();
            }
        }
    }

    pub fn right(&mut self, buffer: &EditorBuffer) {
        if let Some(line) = buffer.get_line(self.cy) {
            if self.cx < line.len() {
                self.cx += 1;
            } else if self.cx == line.len() {
                self.cy += 1;
                self.cx = 0;
            }
        }
    }

    pub fn page_up(&mut self, buffer: &EditorBuffer) {
        self.cy = self.offset_y;
        for _ in 0..self.height {
            self.up(buffer);
        }
    }

    pub fn page_down(&mut self, buffer: &EditorBuffer) {
        self.cy = self.offset_y + self.height - 1;
        for _ in 0..self.height {
            self.down(buffer);
        }
    }

    pub fn home(&mut self, _: &EditorBuffer) {
        self.cx = 0;
    }

    pub fn end(&mut self, buffer: &EditorBuffer) {
        if self.cy < buffer.len() {
            if let Some(line) = buffer.get_line(self.cy) {
                self.cx = line.len();
            }
        }
    }

    pub fn insert_new_line(&mut self, buffer: &mut EditorBuffer) {
        if self.cx == 0 {
            buffer.insert_line(self.cy, "".to_string());
        } else if let Some(current) = buffer.get_line(self.cy) {
            buffer.replace_line(self.cy, (current[0..self.cx]).to_string());
            buffer.insert_line(self.cy + 1, (current[self.cx..]).to_string());
        }
        buffer.clear_highlight(self.cy);
        self.cx = 0;
        self.cy += 1;
    }

    pub fn insert_char(&mut self, buffer: &mut EditorBuffer, c: char) {
        if self.cy == buffer.len() {
            buffer.insert_line(buffer.len(), "".to_string());
            self.cx = 0;
        }
        buffer.insert_char(self.cx, self.cy, c);
        self.cx += 1
    }

    pub fn delete_char(&mut self, buffer: &mut EditorBuffer) {
        if self.cx == 0 && self.cy == 0 {
        } else if self.cx == 0 {
            if let Some(prev) = buffer.get_line(self.cy - 1) {
                if let Some(current) = buffer.get_line(self.cy) {
                    self.cx = prev.len();
                    buffer.append_string(self.cx, self.cy - 1, current);
                    buffer.delete_line(self.cy);
                    self.cy -= 1;
                }
            }
        } else {
            buffer.delete_char(self.cx - 1, self.cy);
            self.cx -= 1;
        }
    }

    pub fn find(&mut self, query: &str, buffer: &mut EditorBuffer) -> bool {
        for i in self.cy..buffer.len() {
            if let Some(line) = buffer.get_line(i) {
                let begin = if i == self.cy { self.cx } else { 0 };

                if let Some(j) = line[begin..line.len()].find(query) {
                    self.cx = begin + j;
                    self.cy = i;
                    return true;
                }
            }
        }
        false
    }

    pub fn rfind(&mut self, query: &str, buffer: &mut EditorBuffer) -> bool {
        for i in (0..=self.cy).rev() {
            if let Some(line) = buffer.get_line(i) {
                let end = if i == self.cy { self.cx } else { line.len() };

                if let Some(j) = line[0..end].rfind(query) {
                    self.cx = j;
                    self.cy = i;
                    return true;
                }
            }
        }
        false
    }

    pub fn adjust(&mut self, buffer: &EditorBuffer) {
        self.rx = 0;

        if let Some(line) = buffer.get_line(self.cy) {
            if line.len() < self.cx {
                self.cx = line.len();
            }
        }

        if self.cy < buffer.len() {
            self.rx = buffer.cx_to_rx(self.cx, self.cy);
        }

        if self.rx < self.offset_x {
            self.offset_x = self.rx;
        }
        if self.rx >= self.offset_x + self.width {
            self.offset_x = self.rx - self.width + 1;
        }

        if self.cy < self.offset_y {
            self.offset_y = self.cy;
        }
        if self.cy >= self.offset_y + self.height {
            self.offset_y = self.cy - self.height + 1;
        }
    }
}

impl Default for EditorScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq)]
pub struct MessageBar {
    message: String,
    updated_at: SystemTime,
}

impl MessageBar {
    pub fn new(message: String, time: SystemTime) -> MessageBar {
        MessageBar {
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
}

impl Drawable for MessageBar {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        buf.push_str("\x1b[K");

        let now = SystemTime::now();
        if let Some(message) = self.get_message(now) {
            buf.push_str(&message);
        }

        Ok(())
    }
}

pub fn refresh_screen(
    screen: &EditorScreen,
    buffer: &EditorBuffer,
    message_bar: &MessageBar,
) -> Result<(), Error> {
    let mut buf = String::new();
    let rows = Rows { screen, buffer };
    let status_bar = StatusBar { screen, buffer };

    buf.push_str("\x1b[?25l");
    buf.push_str("\x1b[H");

    rows.draw(&mut buf)?;
    status_bar.draw(&mut buf)?;
    message_bar.draw(&mut buf)?;

    let cursor = format!(
        "\x1b[{};{}H",
        (screen.cy - screen.offset_y) + 1,
        (screen.rx - screen.offset_x) + 1
    );
    buf.push_str(&cursor);

    buf.push_str("\x1b[?25h");

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

struct Rows<'a> {
    screen: &'a EditorScreen,
    buffer: &'a EditorBuffer,
}

impl Drawable for Rows<'_> {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        for i in 0..self.screen.height {
            let file_line_no = i + self.screen.offset_y;

            if file_line_no < self.buffer.len() {
                if let Some(render) =
                    self.buffer
                        .get_render(file_line_no, self.screen.offset_x, self.screen.width)
                {
                    buf.push_str(&render);
                }
            } else if self.buffer.is_empty() && i == self.screen.height / 3 {
                let title = format!("kilo-rs -- version {}", KILO_VERSION);
                let t: String = title.chars().take(self.screen.width).collect();
                let mut padding = (self.screen.width - t.len()) / 2;
                if padding > 0 {
                    buf.push('~');
                    padding -= 1;
                }
                for _ in 0..padding {
                    buf.push(' ');
                }
                buf.push_str(&t);
            } else {
                buf.push('~');
            }

            buf.push_str("\x1b[K");
            buf.push_str("\r\n");
        }

        Ok(())
    }
}

struct StatusBar<'a> {
    screen: &'a EditorScreen,
    buffer: &'a EditorBuffer,
}

impl Drawable for StatusBar<'_> {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        buf.push_str("\x1b[7m");

        let status = format!(
            "{:<20} - {} lines {}",
            self.buffer
                .get_filepath()
                .unwrap_or_else(|| "[No Name]".to_string()),
            self.screen.height,
            if self.buffer.is_dirty() {
                "(modified)"
            } else {
                ""
            }
        );

        if self.screen.width < status.len() {
            let s: String = status.chars().take(self.screen.width).collect();
            buf.push_str(&s);
        } else {
            buf.push_str(&status);

            let right_status = format!(
                "{} | {}/{}",
                self.buffer
                    .get_file_type()
                    .map_or("no ft", |ft| ft.to_str()),
                self.screen.cy + 1,
                self.buffer.len()
            );
            if self.screen.width as isize - status.len() as isize - right_status.len() as isize > 0
            {
                for _ in 0..(self.screen.width - status.len() - right_status.len()) {
                    buf.push(' ');
                }
                buf.push_str(&right_status);
            } else {
                for _ in 0..(self.screen.width - status.len()) {
                    buf.push(' ');
                }
            }
        }

        buf.push_str("\x1b[m");
        buf.push_str("\r\n");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorBuffer, EditorScreen};

    #[test]
    fn test_adjust() {
        let mut buffer = EditorBuffer::new();
        let mut text = "*".to_string().repeat(100);
        text.push_str("\r\n");
        buffer.load_string(text.repeat(100));

        let mut screen = EditorScreen::new();
        screen.width = 20;
        screen.height = 20;

        screen.cx = 200;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(100, screen.cx);
        assert_eq!(100, screen.rx);
        assert_eq!(81, screen.offset_x);

        screen.cx = 10;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(10, screen.rx);
        assert_eq!(0, screen.offset_x);

        screen.cx = 10;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 50;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(10, screen.rx);
        assert_eq!(10, screen.offset_x);

        screen.cx = 50;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(50, screen.rx);
        assert_eq!(31, screen.offset_x);

        screen.cx = 0;
        screen.cy = 10;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 50;
        screen.adjust(&buffer);
        assert_eq!(10, screen.offset_y);

        screen.cx = 0;
        screen.cy = 50;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(31, screen.offset_y);
    }
}
