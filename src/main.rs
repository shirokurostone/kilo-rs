mod buffer;
mod screen;

use crate::buffer::{EditorBuffer, Highlight};
use crate::screen::{refresh_screen, EditorScreen, MessageBar};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{stdin, stdout, Error, Read, Write};
use std::time::SystemTime;

const KILO_VERSION: &str = "0.1.0";
const TAB_STOP: usize = 8;
const QUIT_TIMES: usize = 3;

#[derive(Debug, PartialEq)]
struct EditorConfig {
    screen: EditorScreen,
    buffer: EditorBuffer,
    message_bar: MessageBar,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Err(err) = run(args) {
        eprintln!("{}", err);
    }
}

fn init_editor() -> Result<EditorConfig, Error> {
    let mut screen = EditorScreen::new();
    screen.init_screen_size()?;

    Ok(EditorConfig {
        screen,
        buffer: EditorBuffer::new(),
        message_bar: MessageBar::new("HELP: Ctrl+Q = quit".to_string(), SystemTime::now()),
    })
}

fn read_single_key(reader: &mut dyn Read) -> Result<char, Error> {
    let mut buf = [0u8; 1];

    loop {
        match reader.read(&mut buf)? {
            0 => continue,
            _ => return Ok(buf[0] as char),
        }
    }
}

fn read_editor_key(reader: &mut dyn Read) -> Result<EditorKey, Error> {
    let c = read_single_key(reader)?;
    let escape_sequence_table = [
        ("\x1b[A", EditorKey::ArrowUp),
        ("\x1b[B", EditorKey::ArrowDown),
        ("\x1b[C", EditorKey::ArrowRight),
        ("\x1b[D", EditorKey::ArrowLeft),
        ("\x1b[H", EditorKey::Home),
        ("\x1b[F", EditorKey::End),
        ("\x1b[1~", EditorKey::Home),
        ("\x1b[3~", EditorKey::Delete),
        ("\x1b[4~", EditorKey::End),
        ("\x1b[5~", EditorKey::PageUp),
        ("\x1b[6~", EditorKey::PageDown),
        ("\x1b[7~", EditorKey::Home),
        ("\x1b[8~", EditorKey::End),
        ("\x1bOH", EditorKey::Home),
        ("\x1bOF", EditorKey::End),
    ];

    match c {
        '\r' => Ok(EditorKey::Enter),
        '\x01'..'\x1b' => Ok(EditorKey::ControlSequence(((c as u8) + b'a' - 1) as char)),
        '\x1b' => {
            let mut buf = String::from("\x1b");
            loop {
                let c2 = read_single_key(reader)?;
                buf.push(c2);

                let matches = escape_sequence_table
                    .iter()
                    .filter(|seq| seq.0.starts_with(&buf))
                    .collect::<Vec<_>>();

                if matches.is_empty() {
                    return Ok(EditorKey::Escape);
                } else if matches.len() == 1 && buf.eq(matches[0].0) {
                    return Ok(matches[0].1);
                }
            }
        }
        '\x7f' => Ok(EditorKey::Backspace),
        c => Ok(EditorKey::NormalKey(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::{read_editor_key, resolve_command, Command, EditorKey};
    use std::io::BufReader;

    fn assert_read_editor_key(input: &str, expected: EditorKey) {
        let data = input.bytes().collect::<Vec<u8>>();
        let mut reader = BufReader::new(&data[..]);
        let actual = read_editor_key(&mut reader);
        assert_eq!(expected, actual.unwrap(), "input:{}", input.escape_debug());
    }

    #[test]
    fn test_read_editor_key_escape() {
        assert_read_editor_key("\x1b[A", EditorKey::ArrowUp);
        assert_read_editor_key("\x1b[B", EditorKey::ArrowDown);
        assert_read_editor_key("\x1b[C", EditorKey::ArrowRight);
        assert_read_editor_key("\x1b[D", EditorKey::ArrowLeft);
        assert_read_editor_key("\x1b[H", EditorKey::Home);
        assert_read_editor_key("\x1b[F", EditorKey::End);

        assert_read_editor_key("\x1b[1~", EditorKey::Home);
        assert_read_editor_key("\x1b[3~", EditorKey::Delete);
        assert_read_editor_key("\x1b[4~", EditorKey::End);
        assert_read_editor_key("\x1b[5~", EditorKey::PageUp);
        assert_read_editor_key("\x1b[6~", EditorKey::PageDown);
        assert_read_editor_key("\x1b[7~", EditorKey::Home);
        assert_read_editor_key("\x1b[8~", EditorKey::End);

        assert_read_editor_key("\x1bOH", EditorKey::Home);
        assert_read_editor_key("\x1bOF", EditorKey::End);
    }

    #[test]
    fn test_read_editor_key() {
        assert_read_editor_key("\r", EditorKey::Enter);
        assert_read_editor_key("\x7f", EditorKey::Backspace);
        assert_read_editor_key(" ", EditorKey::NormalKey(' '));
        assert_read_editor_key("~", EditorKey::NormalKey('~'));
        assert_read_editor_key("\x01", EditorKey::ControlSequence('a'));
        assert_read_editor_key("\x1a", EditorKey::ControlSequence('z'));
    }

    #[test]
    fn test_resolve_command_control_sequence() {
        assert_eq!(
            Command::Noop,
            resolve_command(EditorKey::ControlSequence('a'))
        );
        assert_eq!(
            Command::Backspace,
            resolve_command(EditorKey::ControlSequence('h'))
        );
        assert_eq!(
            Command::Enter,
            resolve_command(EditorKey::ControlSequence('m'))
        );
        assert_eq!(
            Command::Exit,
            resolve_command(EditorKey::ControlSequence('q'))
        );
        assert_eq!(
            Command::Save,
            resolve_command(EditorKey::ControlSequence('s'))
        );
        assert_eq!(
            Command::Find,
            resolve_command(EditorKey::ControlSequence('f'))
        );
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Direction {
    Up,
    Down,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum EditorKey {
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
    ControlSequence(char),
    NormalKey(char),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Command {
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

fn resolve_command(key: EditorKey) -> Command {
    match key {
        EditorKey::ControlSequence('f') => Command::Find,
        EditorKey::ControlSequence('h') => Command::Backspace,
        EditorKey::ControlSequence('m') => Command::Enter,
        EditorKey::ControlSequence('q') => Command::Exit,
        EditorKey::ControlSequence('s') => Command::Save,
        EditorKey::ArrowLeft => Command::ArrowLeft,
        EditorKey::ArrowRight => Command::ArrowRight,
        EditorKey::ArrowUp => Command::ArrowUp,
        EditorKey::ArrowDown => Command::ArrowDown,
        EditorKey::PageUp => Command::PageUp,
        EditorKey::PageDown => Command::PageDown,
        EditorKey::Home => Command::Home,
        EditorKey::End => Command::End,
        EditorKey::Enter => Command::Enter,
        EditorKey::Delete => Command::Delete,
        EditorKey::Backspace => Command::Backspace,
        EditorKey::Escape => Command::Escape,
        EditorKey::NormalKey(c) => Command::Input(c),
        _ => Command::Noop,
    }
}

fn process_key_press(
    reader: &mut dyn Read,
    screen: &mut EditorScreen,
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
    quit_times: &mut usize,
    command: Command,
) -> Result<(), Error> {
    match command {
        Command::Exit => {
            if buffer.is_dirty() && *quit_times > 0 {
                let warning_message = format!(
                    "WARNING!!! File has unsaved changes. Press Ctrl+Q {} more times to quit.",
                    quit_times
                );
                message_bar.set(warning_message, SystemTime::now());
                *quit_times -= 1;
                return Ok(());
            }

            return Err(Error::other("exit"));
        }
        Command::Save => {
            let mut callback =
                |_: &str, _: EditorKey, _: &mut EditorScreen, _: &mut EditorBuffer| {};

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
        }
        Command::Find => {
            let mut direction = Direction::Down;
            let mut last_match = true;
            let mut callback = |query: &str,
                                key: EditorKey,
                                screen: &mut EditorScreen,
                                buffer: &mut EditorBuffer| {
                match key {
                    EditorKey::ArrowUp | EditorKey::ArrowLeft => {
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
                    EditorKey::ArrowDown | EditorKey::ArrowRight => {
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
                    return Ok(());
                }
            }
        }
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
    T: FnMut(&str, EditorKey, &mut EditorScreen, &mut EditorBuffer),
{
    let mut input = String::new();
    let mut buf = String::from(prompt);

    message_bar.set(buf.clone(), SystemTime::now());

    loop {
        refresh_screen(screen, buffer, message_bar)?;
        match read_editor_key(reader)? {
            EditorKey::Enter => {
                message_bar.set("".to_string(), SystemTime::now());
                callback(&input, EditorKey::Enter, screen, buffer);
                return Ok(input);
            }
            EditorKey::Escape => {
                message_bar.set("aborted".to_string(), SystemTime::now());
                callback(&input, EditorKey::Escape, screen, buffer);
                return Err(Error::other("aborted"));
            }
            EditorKey::NormalKey(c) => {
                input.push(c);
                buf.push(c);
                message_bar.set(buf.clone(), SystemTime::now());
                callback(&input, EditorKey::NormalKey(c), screen, buffer);
            }
            key => {
                message_bar.set(buf.clone(), SystemTime::now());
                callback(&input, key, screen, buffer);
            }
        }
    }
}

fn run(args: Vec<String>) -> Result<(), Error> {
    let mut stdin = stdin();
    let mut config = init_editor()?;
    let mut quit_times = QUIT_TIMES;

    if args.len() > 1 {
        config.buffer.load_file(args.get(1).unwrap().to_string())?;
    }

    enable_raw_mode()?;

    loop {
        refresh_screen(&config.screen, &config.buffer, &config.message_bar)?;
        let key = read_editor_key(&mut stdin)?;
        let command = resolve_command(key);
        match process_key_press(
            &mut stdin,
            &mut config.screen,
            &mut config.buffer,
            &mut config.message_bar,
            &mut quit_times,
            command,
        ) {
            Err(_) => break,
            Ok(_) => continue,
        }
    }

    print!("\x1b[2J");
    print!("\x1b[H");
    stdout().flush()?;
    disable_raw_mode()?;

    Ok(())
}
