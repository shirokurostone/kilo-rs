use crate::buffer::EditorBuffer;
use crate::escape_sequence::{
    move_cursor, ESCAPE_SEQUENCE_CLEAR_LINE, ESCAPE_SEQUENCE_HIDE_CURSOR,
    ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION, ESCAPE_SEQUENCE_SHOW_CURSOR,
    ESCAPE_SEQUENCE_STYLE_RESET, ESCAPE_SEQUENCE_STYLE_REVERSE,
};
use crate::KILO_VERSION;
use std::io::{stdout, Error, Write};
use std::time::SystemTime;

pub struct Terminal {
    width: usize,
    height: usize,
}

impl Terminal {
    pub fn new() -> Result<Terminal, Error> {
        let mut terminal = Terminal {
            width: 0,
            height: 0,
        };
        terminal.update()?;
        Ok(terminal)
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        let size = crossterm::terminal::size()?;
        let width = size.0 as usize;
        let height = size.1 as usize;

        let updated = width != self.width || height != self.height;

        self.width = width;
        self.height = height;

        Ok(updated)
    }
}

#[derive(Debug, PartialEq)]
struct Component {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Component {
    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }
}

trait Drawable {
    fn draw(&self, buf: &mut String) -> Result<(), Error>;
}

#[derive(Debug, PartialEq)]
pub struct EditorScreen {
    component: Component,
    buffer: EditorBuffer,
    cx: usize,
    cy: usize,
    rx: usize,
    offset_x: usize,
    offset_y: usize,
}

impl EditorScreen {
    pub fn new() -> EditorScreen {
        EditorScreen {
            component: Component {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            buffer: EditorBuffer::new(),
            cx: 0,
            cy: 0,
            rx: 0,
            offset_x: 0,
            offset_y: 0,
        }
    }

    pub fn buffer(&mut self) -> &mut EditorBuffer {
        &mut self.buffer
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

    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.component.set_size(x, y, width, height);
    }

    pub fn down(&mut self) {
        if !self.buffer.is_empty() && self.cy < self.buffer.len() {
            self.cy += 1;
        }
    }

    pub fn up(&mut self) {
        if self.cy > 0 {
            self.cy -= 1;
        }
    }

    pub fn left(&mut self) {
        if self.cx > 0 {
            self.cx -= 1;
        } else if self.cy > 0 {
            if let Some(line) = self.buffer.get_line(self.cy - 1) {
                self.cy -= 1;
                self.cx = line.len();
            }
        }
    }

    pub fn right(&mut self) {
        if let Some(line) = self.buffer.get_line(self.cy) {
            if self.cx < line.len() {
                self.cx += 1;
            } else if self.cx == line.len() {
                self.cy += 1;
                self.cx = 0;
            }
        }
    }

    pub fn page_up(&mut self) {
        self.cy = self.offset_y;
        for _ in 0..self.component.height {
            self.up();
        }
    }

    pub fn page_down(&mut self) {
        self.cy = self.offset_y + self.component.height - 1;
        for _ in 0..self.component.height {
            self.down();
        }
    }

    pub fn home(&mut self) {
        self.cx = 0;
    }

    pub fn end(&mut self) {
        if self.cy < self.buffer.len() {
            if let Some(line) = self.buffer.get_line(self.cy) {
                self.cx = line.len();
            }
        }
    }

    pub fn insert_new_line(&mut self) {
        if self.cx == 0 {
            self.buffer.insert_line(self.cy, "".to_string());
        } else if let Some(current) = self.buffer.get_line(self.cy) {
            self.buffer
                .replace_line(self.cy, (current[0..self.cx]).to_string());
            self.buffer
                .insert_line(self.cy + 1, (current[self.cx..]).to_string());
        }
        self.buffer.clear_highlight(self.cy);
        self.cx = 0;
        self.cy += 1;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cy == self.buffer.len() {
            self.buffer.insert_line(self.buffer.len(), "".to_string());
            self.cx = 0;
        }
        self.buffer.insert_char(self.cx, self.cy, c);
        self.cx += 1
    }

    pub fn delete_char(&mut self) {
        if self.cx == 0 && self.cy == 0 {
        } else if self.cx == 0 {
            if let Some(prev) = self.buffer.get_line(self.cy - 1) {
                if let Some(current) = self.buffer.get_line(self.cy) {
                    self.cx = prev.len();
                    self.buffer.append_string(self.cx, self.cy - 1, current);
                    self.buffer.delete_line(self.cy);
                    self.cy -= 1;
                }
            }
        } else {
            self.buffer.delete_char(self.cx - 1, self.cy);
            self.cx -= 1;
        }
    }

    pub fn find(&mut self, query: &str) -> bool {
        for i in self.cy..self.buffer.len() {
            if let Some(line) = self.buffer.get_line(i) {
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

    pub fn rfind(&mut self, query: &str) -> bool {
        for i in (0..=self.cy).rev() {
            if let Some(line) = self.buffer.get_line(i) {
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

    pub fn adjust(&mut self) {
        self.rx = 0;

        if let Some(line) = self.buffer.get_line(self.cy) {
            if line.len() < self.cx {
                self.cx = line.len();
            }
        }

        if self.cy < self.buffer.len() {
            self.rx = self.buffer.cx_to_rx(self.cx, self.cy);
        }

        if self.rx < self.offset_x {
            self.offset_x = self.rx;
        }
        if self.rx >= self.offset_x + self.component.width {
            self.offset_x = self.rx - self.component.width + 1;
        }

        if self.cy < self.offset_y {
            self.offset_y = self.cy;
        }
        if self.cy >= self.offset_y + self.component.height {
            self.offset_y = self.cy - self.component.height + 1;
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
    component: Component,
    message: String,
    updated_at: SystemTime,
}

impl MessageBar {
    pub fn new(message: String, time: SystemTime) -> MessageBar {
        MessageBar {
            component: Component {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
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
        let cursor = move_cursor(self.component.x, self.component.y);
        buf.push_str(&cursor);

        buf.push_str(ESCAPE_SEQUENCE_CLEAR_LINE);

        let now = SystemTime::now();
        if let Some(message) = self.get_message(now) {
            buf.push_str(&message);
        }

        Ok(())
    }
}

pub fn refresh_screen(screen: &EditorScreen, message_bar: &MessageBar) -> Result<(), Error> {
    let mut buf = String::new();
    let status_bar = StatusBar {
        component: Component {
            x: 0,
            y: screen.component.height,
            width: screen.component.width,
            height: 1,
        },
        screen,
    };

    buf.push_str(ESCAPE_SEQUENCE_HIDE_CURSOR);
    buf.push_str(ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION);

    screen.draw(&mut buf)?;
    status_bar.draw(&mut buf)?;
    message_bar.draw(&mut buf)?;

    let cursor = move_cursor(
        screen.component.x + screen.rx - screen.offset_x,
        screen.component.y + screen.cy - screen.offset_y,
    );
    buf.push_str(&cursor);

    buf.push_str(ESCAPE_SEQUENCE_SHOW_CURSOR);

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

impl Drawable for EditorScreen {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        for i in 0..self.component.height {
            let file_line_no = i + self.offset_y;

            let cursor = move_cursor(self.component.x, i + self.component.y);
            buf.push_str(&cursor);

            if file_line_no < self.buffer.len() {
                if let Some(render) =
                    self.buffer
                        .get_render(file_line_no, self.offset_x, self.component.width)
                {
                    buf.push_str(&render);
                }
            } else if self.buffer.is_empty() && i == self.component.height / 3 {
                let title = format!("kilo-rs -- version {}", KILO_VERSION);
                let t: String = title.chars().take(self.component.width).collect();
                let mut padding = (self.component.width - t.len()) / 2;
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

            buf.push_str(ESCAPE_SEQUENCE_CLEAR_LINE);
            buf.push_str("\r\n");
        }

        Ok(())
    }
}

struct StatusBar<'a> {
    component: Component,
    screen: &'a EditorScreen,
}

impl StatusBar<'_> {
    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.component.set_size(x, y, width, height);
    }
}

impl Drawable for StatusBar<'_> {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        let cursor = move_cursor(self.component.x, self.component.y);
        buf.push_str(&cursor);

        buf.push_str(ESCAPE_SEQUENCE_STYLE_REVERSE);

        let status = format!(
            "{:<20} - {} lines {}",
            self.screen
                .buffer
                .get_filepath()
                .unwrap_or_else(|| "[No Name]".to_string()),
            self.component.height,
            if self.screen.buffer.is_dirty() {
                "(modified)"
            } else {
                ""
            }
        );

        if self.component.width < status.len() {
            let s: String = status.chars().take(self.component.width).collect();
            buf.push_str(&s);
        } else {
            buf.push_str(&status);

            let right_status = format!(
                "{} | {}/{}",
                self.screen
                    .buffer
                    .get_file_type()
                    .map_or("no ft", |ft| ft.to_str()),
                self.screen.cy + 1,
                self.screen.buffer.len()
            );
            if self.component.width as isize - status.len() as isize - right_status.len() as isize
                > 0
            {
                for _ in 0..(self.component.width - status.len() - right_status.len()) {
                    buf.push(' ');
                }
                buf.push_str(&right_status);
            } else {
                for _ in 0..(self.component.width - status.len()) {
                    buf.push(' ');
                }
            }
        }

        buf.push_str(ESCAPE_SEQUENCE_STYLE_RESET);
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
        screen.component.set_size(0, 0, 20, 20);

        screen.cx = 200;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust();
        assert_eq!(100, screen.cx);
        assert_eq!(100, screen.rx);
        assert_eq!(81, screen.offset_x);

        screen.cx = 10;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust();
        assert_eq!(10, screen.rx);
        assert_eq!(0, screen.offset_x);

        screen.cx = 10;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 50;
        screen.offset_y = 0;
        screen.adjust();
        assert_eq!(10, screen.rx);
        assert_eq!(10, screen.offset_x);

        screen.cx = 50;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust();
        assert_eq!(50, screen.rx);
        assert_eq!(31, screen.offset_x);

        screen.cx = 0;
        screen.cy = 10;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 50;
        screen.adjust();
        assert_eq!(10, screen.offset_y);

        screen.cx = 0;
        screen.cy = 50;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust();
        assert_eq!(31, screen.offset_y);
    }
}
