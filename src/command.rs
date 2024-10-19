use crate::buffer::{EditorBuffer, Highlight};
use crate::key::{read_key, Key};
use crate::screen::{refresh_screen, EditorScreen, MessageBar};
use crate::QUIT_TIMES;
use std::io::{Error, Read};
use std::time::SystemTime;

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

pub fn resolve_command(key: Key) -> Command {
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

#[cfg(test)]
mod tests {
    use crate::command::{resolve_command, Command};
    use crate::key::Key;

    #[test]
    fn test_resolve_command_control_sequence() {
        assert_eq!(Command::Noop, resolve_command(Key::ControlSequence('a')));
        assert_eq!(
            Command::Backspace,
            resolve_command(Key::ControlSequence('h'))
        );
        assert_eq!(Command::Enter, resolve_command(Key::ControlSequence('m')));
        assert_eq!(Command::Exit, resolve_command(Key::ControlSequence('q')));
        assert_eq!(Command::Save, resolve_command(Key::ControlSequence('s')));
        assert_eq!(Command::Find, resolve_command(Key::ControlSequence('f')));
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Direction {
    Up,
    Down,
}

fn process_exit_command(
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
    quit_times: &mut usize,
) -> Result<(), Error> {
    if buffer.is_dirty() && *quit_times > 0 {
        let warning_message = format!(
            "WARNING!!! File has unsaved changes. Press Ctrl+Q {} more times to quit.",
            quit_times
        );
        message_bar.set(warning_message, SystemTime::now());
        *quit_times -= 1;
        return Ok(());
    }

    Err(Error::other("exit"))
}

fn process_save_command(
    reader: &mut dyn Read,
    screen: &mut EditorScreen,
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
) -> Result<(), Error> {
    let mut callback = |_: &str, _: Key, _: &mut EditorScreen, _: &mut EditorBuffer| {};

    let ret = if buffer.get_filepath().is_none() {
        match prompt(
            reader,
            screen,
            buffer,
            message_bar,
            "Save as: ",
            &mut callback,
        ) {
            Ok(path) => buffer.save_file(path),
            Err(_) => return Ok(()),
        }
    } else {
        buffer.overwrite_file()
    };

    match ret {
        Ok(size) => {
            let success_message = format!("{} bytes written to disk", size);
            message_bar.set(success_message, SystemTime::now());
        }
        Err(err) => {
            let err_message = format!("Can't save! I/O error: {}", err);
            message_bar.set(err_message, SystemTime::now());
        }
    }

    Ok(())
}

fn process_find_command(
    reader: &mut dyn Read,
    screen: &mut EditorScreen,
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
) -> Result<(), Error> {
    let mut direction = Direction::Down;
    let mut last_match = true;
    let mut callback =
        |query: &str, key: Key, screen: &mut EditorScreen, buffer: &mut EditorBuffer| match key {
            Key::ArrowUp | Key::ArrowLeft => {
                direction = Direction::Up;
                if !last_match {
                    if let Some(last_line) = buffer.get_line(buffer.len() - 1) {
                        screen.set_cursor(last_line.len() - 1, buffer.len())
                    }
                }
                let (cx, cy) = screen.cursor();
                screen.left(buffer);
                last_match = screen.rfind(query, buffer);
                if last_match {
                    buffer.clear_highlight(cy);
                    let cur = screen.cursor();
                    buffer.highlight(cur.0, cur.1, query.len(), Highlight::Match);
                } else {
                    screen.set_cursor(cx, cy);
                }
                screen.adjust(buffer);
            }
            Key::ArrowDown | Key::ArrowRight => {
                direction = Direction::Down;
                if !last_match {
                    screen.set_cursor(0, 0);
                }
                let (cx, cy) = screen.cursor();
                screen.right(buffer);
                last_match = screen.find(query, buffer);
                if last_match {
                    buffer.clear_highlight(cy);
                    let cur = screen.cursor();
                    buffer.highlight(cur.0, cur.1, query.len(), Highlight::Match);
                } else {
                    screen.set_cursor(cx, cy);
                }
                screen.adjust(buffer);
            }
            _ => {
                if !last_match {
                    match direction {
                        Direction::Up => {
                            if let Some(last_line) = buffer.get_line(buffer.len() - 1) {
                                screen.set_cursor(last_line.len() - 1, buffer.len())
                            }
                        }
                        Direction::Down => {
                            screen.set_cursor(0, 0);
                        }
                    }
                }
                let (_, cy) = screen.cursor();
                last_match = match direction {
                    Direction::Up => screen.rfind(query, buffer),
                    Direction::Down => screen.find(query, buffer),
                };
                buffer.clear_highlight(cy);
                if last_match {
                    let cur = screen.cursor();
                    buffer.highlight(cur.0, cur.1, query.len(), Highlight::Match);
                }
                screen.adjust(buffer);
            }
        };
    let (cx, cy) = screen.cursor();
    let (offset_x, offset_y) = screen.offset();

    match prompt(
        reader,
        screen,
        buffer,
        message_bar,
        "Search: ",
        &mut callback,
    ) {
        Ok(_) => {}
        Err(_) => {
            screen.set_cursor(cx, cy);
            screen.set_offset(offset_x, offset_y);
            screen.adjust(buffer);
        }
    }
    Ok(())
}

pub fn process_command(
    reader: &mut dyn Read,
    screen: &mut EditorScreen,
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
    quit_times: &mut usize,
    command: Command,
) -> Result<(), Error> {
    match command {
        Command::Exit => process_exit_command(buffer, message_bar, quit_times)?,
        Command::Save => process_save_command(reader, screen, buffer, message_bar)?,
        Command::Find => process_find_command(reader, screen, buffer, message_bar)?,
        Command::ArrowDown => screen.down(buffer),
        Command::ArrowUp => screen.up(buffer),
        Command::ArrowLeft => screen.left(buffer),
        Command::ArrowRight => screen.right(buffer),
        Command::PageUp => screen.page_up(buffer),
        Command::PageDown => screen.page_down(buffer),
        Command::Home => screen.home(buffer),
        Command::Enter => screen.insert_new_line(buffer),
        Command::End => screen.end(buffer),
        Command::Delete => {
            screen.right(buffer);
            screen.delete_char(buffer);
        }
        Command::Backspace => screen.delete_char(buffer),
        Command::Input(c) => screen.insert_char(buffer, c),
        Command::Escape => {}
        Command::Noop => {}
    }

    screen.adjust(buffer);
    *quit_times = QUIT_TIMES;

    Ok(())
}

pub fn prompt<T>(
    reader: &mut dyn Read,
    screen: &mut EditorScreen,
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
    prompt: &str,
    callback: &mut T,
) -> Result<String, Error>
where
    T: FnMut(&str, Key, &mut EditorScreen, &mut EditorBuffer),
{
    let mut input = String::new();
    let mut buf = String::from(prompt);

    message_bar.set(buf.clone(), SystemTime::now());

    loop {
        refresh_screen(screen, buffer, message_bar)?;
        match read_key(reader)? {
            Key::Enter => {
                message_bar.set("".to_string(), SystemTime::now());
                callback(&input, Key::Enter, screen, buffer);
                return Ok(input);
            }
            Key::Escape => {
                message_bar.set("aborted".to_string(), SystemTime::now());
                callback(&input, Key::Escape, screen, buffer);
                return Err(Error::other("aborted"));
            }
            Key::NormalKey(c) => {
                input.push(c);
                buf.push(c);
                message_bar.set(buf.clone(), SystemTime::now());
                callback(&input, Key::NormalKey(c), screen, buffer);
            }
            key => {
                message_bar.set(buf.clone(), SystemTime::now());
                callback(&input, key, screen, buffer);
            }
        }
    }
}
