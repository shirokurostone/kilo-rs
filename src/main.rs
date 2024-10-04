mod buffer;
mod screen;

use crate::buffer::EditorBuffer;
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
    match read_single_key(reader)? {
        '\x08' => {
            // ctrl+h
            Ok(EditorKey::Backspace)
        }
        '\r' => Ok(EditorKey::Enter),
        '\x11' => {
            // ctrl+q
            Ok(EditorKey::Exit)
        }
        '\x13' => {
            // ctrl+s
            Ok(EditorKey::Save)
        }
        '\x1b' => match read_single_key(reader)? {
            '[' => match read_single_key(reader)? {
                'A' => Ok(EditorKey::ArrowUp),
                'B' => Ok(EditorKey::ArrowDown),
                'C' => Ok(EditorKey::ArrowRight),
                'D' => Ok(EditorKey::ArrowLeft),
                'H' => Ok(EditorKey::Home),
                'F' => Ok(EditorKey::End),
                '1' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::Home),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                '3' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::Delete),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                '4' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::End),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                '5' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::PageUp),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                '6' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::PageDown),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                '7' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::Home),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                '8' => match read_single_key(reader)? {
                    '~' => Ok(EditorKey::End),
                    _ => Ok(EditorKey::OtherKey('\x1b')),
                },
                _ => Ok(EditorKey::OtherKey('\x1b')),
            },
            'O' => match read_single_key(reader)? {
                'H' => Ok(EditorKey::Home),
                'F' => Ok(EditorKey::End),
                _ => Ok(EditorKey::OtherKey('\x1b')),
            },
            _ => Ok(EditorKey::OtherKey('\x1b')),
        },
        '\x7f' => Ok(EditorKey::Backspace),
        c => Ok(EditorKey::OtherKey(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::{read_editor_key, EditorKey};
    use std::io::BufReader;

    #[test]
    fn test_read_editor_key() {
        let assert = |input: &str, expected: EditorKey| {
            let data = input.bytes().collect::<Vec<u8>>();
            let mut reader = BufReader::new(&data[..]);
            let actual = read_editor_key(&mut reader);
            assert_eq!(expected, actual.unwrap(), "input:{}", input.escape_debug());
        };

        assert("\x11", EditorKey::Exit);

        assert("\x1b[A", EditorKey::ArrowUp);
        assert("\x1b[B", EditorKey::ArrowDown);
        assert("\x1b[C", EditorKey::ArrowRight);
        assert("\x1b[D", EditorKey::ArrowLeft);
        assert("\x1b[H", EditorKey::Home);
        assert("\x1b[F", EditorKey::End);

        assert("\x1b[1~", EditorKey::Home);
        assert("\x1b[3~", EditorKey::Delete);
        assert("\x1b[4~", EditorKey::End);
        assert("\x1b[5~", EditorKey::PageUp);
        assert("\x1b[6~", EditorKey::PageDown);
        assert("\x1b[7~", EditorKey::Home);
        assert("\x1b[8~", EditorKey::End);

        assert("\x1bOH", EditorKey::Home);
        assert("\x1bOF", EditorKey::End);

        assert("a", EditorKey::OtherKey('a'));
    }
}

#[derive(Debug, PartialEq)]
enum EditorKey {
    Exit,
    Save,
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
    OtherKey(char),
}

fn process_key_press(
    screen: &mut EditorScreen,
    buffer: &mut EditorBuffer,
    message_bar: &mut MessageBar,
    quit_times: &mut usize,
    editor_key: EditorKey,
) -> Result<(), Error> {
    match editor_key {
        EditorKey::Exit => {
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
        EditorKey::Save => match buffer.overwrite_file() {
            Ok(size) => {
                let success_message = format!("{} bytes written to disk", size);
                message_bar.set(success_message, SystemTime::now());
            }
            Err(err) => {
                let err_message = format!("Can't save! I/O error: {}", err);
                message_bar.set(err_message, SystemTime::now());
            }
        },
        EditorKey::ArrowDown => screen.down(buffer),
        EditorKey::ArrowUp => screen.up(buffer),
        EditorKey::ArrowLeft => screen.left(buffer),
        EditorKey::ArrowRight => screen.right(buffer),
        EditorKey::PageUp => screen.page_up(buffer),
        EditorKey::PageDown => screen.page_down(buffer),
        EditorKey::Home => screen.home(buffer),
        EditorKey::Enter => {}
        EditorKey::End => screen.end(buffer),
        EditorKey::Delete => {}
        EditorKey::Backspace => {}
        EditorKey::OtherKey(c) => match c {
            '\x0c' => {} // ctrl+l
            '\x1b' => {} // esc
            key => screen.insert_char(buffer, key),
        },
    }

    screen.adjust(buffer);
    *quit_times = QUIT_TIMES;

    Ok(())
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
        match process_key_press(
            &mut config.screen,
            &mut config.buffer,
            &mut config.message_bar,
            &mut quit_times,
            read_editor_key(&mut stdin)?,
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
