use crate::buffer::EditorBuffer;
use crate::escape_sequence::{
    move_terminal_cursor, ESCAPE_SEQUENCE_CLEAR_LINE, ESCAPE_SEQUENCE_HIDE_CURSOR,
    ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION, ESCAPE_SEQUENCE_SHOW_CURSOR,
};
use crate::pane::Pane;
use crate::ui::{Component, Drawable};
use crate::KILO_VERSION;
use std::io::{stdout, Error, Write};

#[derive(Debug, PartialEq)]
pub struct Screen {
    component: Component,
    buffer: EditorBuffer,
    cx: usize,
    cy: usize,
    rx: usize,
    offset_x: usize,
    offset_y: usize,
}

impl Screen {
    pub fn new() -> Screen {
        Screen {
            component: Component::default(),
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
        for _ in 0..self.component.height() {
            self.up();
        }
    }

    pub fn page_down(&mut self) {
        self.cy = self.offset_y + self.component.height() - 1;
        for _ in 0..self.component.height() {
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
        if self.rx >= self.offset_x + self.component.width() {
            self.offset_x = self.rx - self.component.width() + 1;
        }

        if self.cy < self.offset_y {
            self.offset_y = self.cy;
        }
        if self.cy >= self.offset_y + self.component.height() {
            self.offset_y = self.cy - self.component.height() + 1;
        }
    }

    pub fn get_terminal_cursor(&self) -> (usize, usize) {
        (
            self.component.x() + self.rx - self.offset_x,
            self.component.y() + self.cy - self.offset_y,
        )
    }

    pub fn get_cy(&self) -> usize {
        self.cy
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

pub fn refresh_screen(pane: &mut Pane) -> Result<(), Error> {
    let mut buf = String::new();
    buf.push_str(ESCAPE_SEQUENCE_HIDE_CURSOR);
    buf.push_str(ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION);

    pane.draw(&mut buf)?;

    let cursor = pane.get_terminal_cursor();
    let move_cursor_str = move_terminal_cursor(cursor.0, cursor.1);
    buf.push_str(&move_cursor_str);

    buf.push_str(ESCAPE_SEQUENCE_SHOW_CURSOR);

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

impl Drawable for Screen {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        for i in 0..self.component.height() {
            let file_line_no = i + self.offset_y;

            let cursor = move_terminal_cursor(self.component.x(), i + self.component.y());
            buf.push_str(&cursor);

            if file_line_no < self.buffer.len() {
                if let Some(render) =
                    self.buffer
                        .get_render(file_line_no, self.offset_x, self.component.width())
                {
                    buf.push_str(&render);
                }
            } else if self.buffer.is_empty() && i == self.component.height() / 3 {
                let title = format!("kilo-rs -- version {}", KILO_VERSION);
                let t: String = title.chars().take(self.component.width()).collect();
                let mut padding = (self.component.width() - t.len()) / 2;
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

#[cfg(test)]
mod tests {
    use super::{EditorBuffer, Screen};

    fn initialize_screen() -> Screen {
        let mut screen = Screen::new();
        screen.component.set_size(0, 0, 20, 20);

        let mut text = "*".to_string().repeat(100);
        text.push_str("\r\n");
        screen.buffer.load_string(text.repeat(100));

        screen
    }

    fn cursor_test_runner<T>(test_cases: &[([usize; 2], [usize; 2])], func: T)
    where
        T: Fn(&mut Screen) -> (),
    {
        let mut screen = initialize_screen();
        for (i, data) in test_cases.iter().enumerate() {
            [screen.cx, screen.cy] = data.0;
            func(&mut screen);
            assert_eq!(data.1, [screen.cx, screen.cy], "i={}", i);
        }
    }

    #[test]
    fn test_cursor_left() {
        cursor_test_runner(
            &[([0, 0], [0, 0]), ([1, 0], [0, 0]), ([0, 1], [100, 0])][..],
            |s: &mut Screen| s.left(),
        );
    }

    #[test]
    fn test_cursor_right() {
        cursor_test_runner(
            &[
                ([0, 0], [1, 0]),
                ([100, 0], [0, 1]),
                ([100, 100], [100, 100]),
            ][..],
            |s: &mut Screen| s.right(),
        );
    }

    #[test]
    fn test_cursor_up() {
        cursor_test_runner(
            &[([0, 0], [0, 0]), ([0, 1], [0, 0])][..],
            |s: &mut Screen| s.up(),
        );
    }

    #[test]
    fn test_cursor_down() {
        cursor_test_runner(
            &[([0, 0], [0, 1]), ([0, 100], [0, 100])][..],
            |s: &mut Screen| s.down(),
        );
    }

    #[test]
    fn test_cursor_home() {
        cursor_test_runner(
            &[([0, 0], [0, 0]), ([100, 0], [0, 0])][..],
            |s: &mut Screen| s.home(),
        );
    }

    #[test]
    fn test_cursor_end() {
        cursor_test_runner(
            &[([0, 0], [100, 0]), ([100, 0], [100, 0])][..],
            |s: &mut Screen| s.end(),
        );
    }

    #[test]
    fn test_adjust() {
        let mut screen = initialize_screen();

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
