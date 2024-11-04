use crate::buffer::{EditorBuffer, Highlight};
use crate::escape_sequence::{
    move_cursor, ESCAPE_SEQUENCE_CLEAR_LINE, ESCAPE_SEQUENCE_HIDE_CURSOR,
    ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION, ESCAPE_SEQUENCE_SHOW_CURSOR,
    ESCAPE_SEQUENCE_STYLE_RESET, ESCAPE_SEQUENCE_STYLE_REVERSE,
};
use crate::key::{read_key, Key};
use crate::{KILO_VERSION, QUIT_TIMES};
use std::io::{stdout, Error, Read, Write};
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
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Component {
        Component {
            x,
            y,
            width,
            height,
        }
    }
    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }
}

impl Default for Component {
    fn default() -> Self {
        Component::new(0, 0, 0, 0)
    }
}

trait Drawable {
    fn draw(&self, buf: &mut String) -> Result<(), Error>;
}

pub struct UiGroup {
    component: Component,
    screen: EditorScreen,
    status_bar: StatusBar,
    message_bar: MessageBar,
    quit_times: usize,
}

impl UiGroup {
    pub fn new(message: String, system_time: SystemTime) -> UiGroup {
        UiGroup {
            component: Component::default(),
            screen: EditorScreen::new(),
            status_bar: StatusBar {
                component: Component::default(),
                left_status: "".to_string(),
                right_status: "".to_string(),
            },
            message_bar: MessageBar::new(message, system_time),
            quit_times: QUIT_TIMES,
        }
    }

    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.component.set_size(x, y, width, height);
        self.screen.set_size(x, y, width, height - 2);
        self.status_bar.set_size(x, y + height - 2, width, 1);
        self.message_bar.set_size(x, y + height - 1, width, 1);
    }

    pub fn screen(&mut self) -> &mut EditorScreen {
        &mut self.screen
    }

    pub fn message_bar(&mut self) -> &mut MessageBar {
        &mut self.message_bar
    }

    pub fn resolve_command(&self, key: Key) -> Command {
        match key {
            Key::ControlSequence('f') => Command::Find,
            Key::ControlSequence('h') => Command::Backspace,
            Key::ControlSequence('m') => Command::Enter,
            Key::ControlSequence('q') => Command::Exit,
            Key::ControlSequence('s') => Command::Save,
            Key::ArrowLeft => Command::ArrowLeft,
            Key::ArrowRight => Command::ArrowRight,
            Key::ArrowUp => Command::ArrowUp,
            Key::ArrowDown => Command::ArrowDown,
            Key::PageUp => Command::PageUp,
            Key::PageDown => Command::PageDown,
            Key::Home => Command::Home,
            Key::End => Command::End,
            Key::Enter => Command::Enter,
            Key::Delete => Command::Delete,
            Key::Backspace => Command::Backspace,
            Key::Escape => Command::Escape,
            Key::NormalKey(c) => Command::Input(c),
            _ => Command::Noop,
        }
    }

    pub fn process_command(
        &mut self,
        reader: &mut dyn Read,
        command: Command,
    ) -> Result<(), Error> {
        match command {
            Command::Exit => self.process_exit_command()?,
            Command::Save => self.process_save_command(reader)?,
            Command::Find => self.process_find_command(reader)?,
            Command::ArrowDown => self.screen.down(),
            Command::ArrowUp => self.screen.up(),
            Command::ArrowLeft => self.screen.left(),
            Command::ArrowRight => self.screen.right(),
            Command::PageUp => self.screen.page_up(),
            Command::PageDown => self.screen.page_down(),
            Command::Home => self.screen.home(),
            Command::Enter => self.screen.insert_new_line(),
            Command::End => self.screen.end(),
            Command::Delete => {
                self.screen.right();
                self.screen.delete_char();
            }
            Command::Backspace => self.screen.delete_char(),
            Command::Input(c) => self.screen.insert_char(c),
            Command::Escape => {}
            Command::Noop => {}
        }

        self.post_process();
        if command != Command::Exit {
            self.quit_times = QUIT_TIMES;
        }

        Ok(())
    }

    fn post_process(&mut self) {
        self.screen.adjust();
        self.status_bar.set_left_status(&self.screen);
        self.status_bar.set_right_status(&self.screen);
    }

    fn get_cursor(&self) -> (usize, usize) {
        (
            self.screen.component.x + self.screen.rx - self.screen.offset_x,
            self.screen.component.y + self.screen.cy - self.screen.offset_y,
        )
    }

    pub fn process_exit_command(&mut self) -> Result<(), Error> {
        let buffer = &mut self.screen.buffer;
        let message_bar = &mut self.message_bar;
        if buffer.is_dirty() && self.quit_times > 0 {
            let warning_message = format!(
                "WARNING!!! File has unsaved changes. Press Ctrl+Q {} more times to quit.",
                self.quit_times
            );
            message_bar.set(warning_message, SystemTime::now());
            self.quit_times -= 1;
            return Ok(());
        }

        Err(Error::other("exit"))
    }

    pub fn process_save_command(&mut self, reader: &mut dyn Read) -> Result<(), Error> {
        let mut callback = |_: &str, _: Key, _: &mut EditorScreen| {};

        let filepath = self.screen.buffer().get_filepath();
        let ret = if filepath.is_none() {
            match self.prompt(reader, "Save as: ", &mut callback) {
                Ok(path) => self.screen.buffer().save_file(path),
                Err(_) => return Ok(()),
            }
        } else {
            self.screen.buffer().overwrite_file()
        };

        match ret {
            Ok(size) => {
                let success_message = format!("{} bytes written to disk", size);
                self.message_bar.set(success_message, SystemTime::now());
            }
            Err(err) => {
                let err_message = format!("Can't save! I/O error: {}", err);
                self.message_bar.set(err_message, SystemTime::now());
            }
        }

        Ok(())
    }

    pub fn process_find_command(&mut self, reader: &mut dyn Read) -> Result<(), Error> {
        let mut direction = Direction::Down;
        let mut last_match = true;
        let mut callback = |query: &str, key: Key, screen: &mut EditorScreen| match key {
            Key::ArrowUp | Key::ArrowLeft => {
                direction = Direction::Up;
                if !last_match {
                    let buffer_len = screen.buffer().len();
                    let buffer_last_line = screen.buffer().get_line(buffer_len - 1);
                    if let Some(last_line) = buffer_last_line {
                        screen.set_cursor(last_line.len() - 1, buffer_len)
                    }
                }
                let (cx, cy) = screen.cursor();
                screen.left();
                last_match = screen.rfind(query);
                if last_match {
                    screen.buffer().clear_highlight(cy);
                    let cur = screen.cursor();
                    screen
                        .buffer()
                        .highlight(cur.0, cur.1, query.len(), Highlight::Match);
                } else {
                    screen.set_cursor(cx, cy);
                }
                screen.adjust();
            }
            Key::ArrowDown | Key::ArrowRight => {
                direction = Direction::Down;
                if !last_match {
                    screen.set_cursor(0, 0);
                }
                let (cx, cy) = screen.cursor();
                screen.right();
                last_match = screen.find(query);
                if last_match {
                    screen.buffer().clear_highlight(cy);
                    let cur = screen.cursor();
                    screen
                        .buffer()
                        .highlight(cur.0, cur.1, query.len(), Highlight::Match);
                } else {
                    screen.set_cursor(cx, cy);
                }
                screen.adjust();
            }
            _ => {
                if !last_match {
                    match direction {
                        Direction::Up => {
                            let buffer_len = screen.buffer().len();
                            let buffer_last_line = screen.buffer().get_line(buffer_len - 1);
                            if let Some(last_line) = buffer_last_line {
                                screen.set_cursor(last_line.len() - 1, buffer_len)
                            }
                        }
                        Direction::Down => {
                            screen.set_cursor(0, 0);
                        }
                    }
                }
                let (_, cy) = screen.cursor();
                last_match = match direction {
                    Direction::Up => screen.rfind(query),
                    Direction::Down => screen.find(query),
                };
                screen.buffer().clear_highlight(cy);
                if last_match {
                    let cur = screen.cursor();
                    screen
                        .buffer()
                        .highlight(cur.0, cur.1, query.len(), Highlight::Match);
                }
                screen.adjust();
            }
        };
        let (cx, cy) = self.screen.cursor();
        let (offset_x, offset_y) = self.screen.offset();

        match self.prompt(reader, "Search: ", &mut callback) {
            Ok(_) => {}
            Err(_) => {
                self.screen.set_cursor(cx, cy);
                self.screen.set_offset(offset_x, offset_y);
                self.screen.adjust();
            }
        }
        Ok(())
    }

    pub fn prompt<T>(
        &mut self,
        reader: &mut dyn Read,
        prompt: &str,
        callback: &mut T,
    ) -> Result<String, Error>
    where
        T: FnMut(&str, Key, &mut EditorScreen),
    {
        let mut input = String::new();
        let mut buf = String::from(prompt);

        self.message_bar.set(buf.clone(), SystemTime::now());

        loop {
            refresh_screen(self);
            match read_key(reader)? {
                Key::Enter => {
                    self.message_bar.set("".to_string(), SystemTime::now());
                    callback(&input, Key::Enter, &mut self.screen);
                    return Ok(input);
                }
                Key::Escape => {
                    self.message_bar
                        .set("aborted".to_string(), SystemTime::now());
                    callback(&input, Key::Escape, &mut self.screen);
                    return Err(Error::other("aborted"));
                }
                Key::NormalKey(c) => {
                    input.push(c);
                    buf.push(c);
                    self.message_bar.set(buf.clone(), SystemTime::now());
                    callback(&input, Key::NormalKey(c), &mut self.screen);
                }
                key => {
                    self.message_bar.set(buf.clone(), SystemTime::now());
                    callback(&input, key, &mut self.screen);
                }
            }
        }
    }
}

impl Drawable for UiGroup {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        self.screen.draw(buf)?;
        self.status_bar.draw(buf)?;
        self.message_bar.draw(buf)?;
        Ok(())
    }
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

pub fn refresh_screen(ui_group: &mut UiGroup) -> Result<(), Error> {
    let mut buf = String::new();
    buf.push_str(ESCAPE_SEQUENCE_HIDE_CURSOR);
    buf.push_str(ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION);

    ui_group.draw(&mut buf)?;

    let cursor = ui_group.get_cursor();
    let move_cursor_str = move_cursor(cursor.0, cursor.1);
    buf.push_str(&move_cursor_str);

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

#[derive(Debug, PartialEq)]
struct StatusBar {
    component: Component,
    left_status: String,
    right_status: String,
}

impl StatusBar {
    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.component.set_size(x, y, width, height);
    }

    pub fn set_left_status(&mut self, screen: &EditorScreen) {
        self.left_status = format!(
            "{:<20} - {} lines {}",
            screen
                .buffer
                .get_filepath()
                .unwrap_or_else(|| "[No Name]".to_string()),
            self.component.height,
            if screen.buffer.is_dirty() {
                "(modified)"
            } else {
                ""
            }
        );
    }

    pub fn set_right_status(&mut self, screen: &EditorScreen) {
        self.right_status = format!(
            "{} | {}/{}",
            screen
                .buffer
                .get_file_type()
                .map_or("no ft", |ft| ft.to_str()),
            screen.cy + 1,
            screen.buffer.len()
        );
    }
}

impl Drawable for StatusBar {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        let cursor = move_cursor(self.component.x, self.component.y);
        buf.push_str(&cursor);

        buf.push_str(ESCAPE_SEQUENCE_STYLE_REVERSE);

        if self.component.width < self.left_status.len() {
            let s: String = self
                .left_status
                .chars()
                .take(self.component.width)
                .collect();
            buf.push_str(&s);
        } else {
            buf.push_str(&self.left_status);

            if self.component.width as isize
                - self.left_status.len() as isize
                - self.right_status.len() as isize
                > 0
            {
                for _ in
                    0..(self.component.width - self.left_status.len() - self.right_status.len())
                {
                    buf.push(' ');
                }
                buf.push_str(&self.right_status);
            } else {
                for _ in 0..(self.component.width - self.left_status.len()) {
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Command {
    Exit,
    Save,
    Find,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Delete,
    Backspace,
    Escape,
    Input(char),
    Noop,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Direction {
    Up,
    Down,
}
