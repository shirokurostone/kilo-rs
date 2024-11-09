use crate::buffer::Highlight;
use crate::key::{read_key, Key};
use crate::message_bar::MessageBar;
use crate::screen::{refresh_screen, Screen};
use crate::status_bar::StatusBar;
use crate::ui::{Component, Drawable};
use crate::QUIT_TIMES;
use std::io::{Error, Read};
use std::time::SystemTime;

pub struct Pane {
    component: Component,
    screen: Screen,
    status_bar: StatusBar,
    message_bar: MessageBar,
    quit_times: usize,
}

impl Pane {
    pub fn new(message: String, system_time: SystemTime) -> Pane {
        Pane {
            component: Component::default(),
            screen: Screen::new(),
            status_bar: StatusBar::new(),
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

    pub fn screen(&mut self) -> &mut Screen {
        &mut self.screen
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
        self.status_bar.set_left_status(&mut self.screen);
        self.status_bar.set_right_status(&mut self.screen);
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        self.screen.get_cursor()
    }

    pub fn process_exit_command(&mut self) -> Result<(), Error> {
        let buffer = &mut self.screen.buffer();
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
        let mut callback = |_: &str, _: Key, _: &mut Screen| {};

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
        let mut callback = |query: &str, key: Key, screen: &mut Screen| match key {
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
        T: FnMut(&str, Key, &mut Screen),
    {
        let mut input = String::new();
        let mut buf = String::from(prompt);

        self.message_bar.set(buf.clone(), SystemTime::now());

        loop {
            refresh_screen(self)?;
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

impl Drawable for Pane {
    fn draw(&self, buf: &mut String) -> Result<(), Error> {
        self.screen.draw(buf)?;
        self.status_bar.draw(buf)?;
        self.message_bar.draw(buf)?;
        Ok(())
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
