use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{stdin, stdout, Error, Read, Write};

const KILO_VERSION: &str = "0.1.0";

struct EditorConfig {
    cx: usize,
    cy: usize,
    screen_width: usize,
    screen_height: usize,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
    }
}

fn init_editor() -> Result<EditorConfig, Error> {
    let size = crossterm::terminal::size()?;

    Ok(EditorConfig {
        cx: 0,
        cy: 0,
        screen_width: size.0 as usize,
        screen_height: size.1 as usize,
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
        '\x11' => {
            // ctrl+q
            Ok(EditorKey::Exit)
        }
        '\x1b' => match read_single_key(reader)? {
            '[' => match read_single_key(reader)? {
                'A' => Ok(EditorKey::ArrowUp),
                'B' => Ok(EditorKey::ArrowDown),
                'C' => Ok(EditorKey::ArrowRight),
                'D' => Ok(EditorKey::ArrowLeft),
                'H' => Ok(EditorKey::Home),
                'F' => Ok(EditorKey::End),
                _ => Ok(EditorKey::OtherKey('\x1b')),
            },
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
            'O' => match read_single_key(reader)? {
                'H' => Ok(EditorKey::Home),
                'F' => Ok(EditorKey::End),
                _ => Ok(EditorKey::OtherKey('\x1b')),
            },
            _ => Ok(EditorKey::OtherKey('\x1b')),
        },
        c => Ok(EditorKey::OtherKey(c)),
    }
}

enum EditorKey {
    Exit,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
    OtherKey(char),
}

fn process_key_press(config: &mut EditorConfig, reader: &mut dyn Read) -> Result<(), Error> {
    match read_editor_key(reader)? {
        EditorKey::Exit => {
            // ctrl+q
            return Err(Error::other("exit"));
        }
        EditorKey::ArrowDown => {
            if config.cy < config.screen_height - 1 {
                config.cy += 1;
            }
        }
        EditorKey::ArrowUp => {
            if config.cy > 0 {
                config.cy -= 1;
            }
        }
        EditorKey::ArrowLeft => {
            if config.cx > 0 {
                config.cx -= 1;
            }
        }
        EditorKey::ArrowRight => {
            if config.cx < config.screen_width - 1 {
                config.cx += 1;
            }
        }
        EditorKey::PageUp => {
            if config.cy < config.screen_height {
                config.cy = 0;
            } else {
                config.cy -= config.screen_height;
            }
            config.cy = 0;
        }
        EditorKey::PageDown => {
            config.cy += config.screen_height;
            if config.cy > config.screen_height - 1 {
                config.cy = config.screen_height - 1;
            }
        }
        EditorKey::Home => {
            config.cx = 0;
        }
        EditorKey::End => {
            config.cx = config.screen_width - 1;
        }
        EditorKey::Delete => {}
        EditorKey::OtherKey(c) => {
            print!("{}\r\n", c);
        }
    }
    Ok(())
}

fn refresh_screen(config: &EditorConfig) -> Result<(), Error> {
    let mut buf = String::new();

    buf.push_str("\x1b[?25l");
    buf.push_str("\x1b[H");

    draw_rows(config, &mut buf)?;

    let cursor = format!("\x1b[{};{}H", config.cy + 1, config.cx + 1);
    buf.push_str(&cursor);

    buf.push_str("\x1b[?25h");

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

fn draw_rows(config: &EditorConfig, buf: &mut String) -> Result<(), Error> {
    for i in 0..config.screen_height {
        if i == config.screen_height / 3 {
            let title = format!("kilo-rs -- version {}", KILO_VERSION);
            let t: String = title.chars().take(config.screen_width).collect();
            let mut padding = (config.screen_width - t.len()) / 2;
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
        if i < config.screen_height - 1 {
            buf.push_str("\r\n");
        }
    }

    Ok(())
}

fn run() -> Result<(), Error> {
    let mut stdin = stdin();
    let mut config = init_editor()?;

    enable_raw_mode()?;

    loop {
        refresh_screen(&config)?;
        match process_key_press(&mut config, &mut stdin) {
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
