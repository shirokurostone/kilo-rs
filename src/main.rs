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

fn read_editor_key(mut reader: impl Read) -> Result<char, Error> {
    let mut buf = [0u8; 1];

    loop {
        match reader.read(&mut buf)? {
            0 => continue,
            _ => return Ok(buf[0] as char),
        }
    }
}

fn process_key_press(reader: impl Read) -> Result<(), Error> {
    match read_editor_key(reader)? {
        '\x11' => {
            // ctrl+q
            return Err(Error::other("exit"));
        }
        c => {
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

    let cursor = format!("\x1b[{}H\x1b[{}d", config.cx + 1, config.cy + 1);
    buf.push_str(&cursor);

    buf.push_str("\x1b[?25h");

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

fn draw_rows(config: &EditorConfig, buf: &mut String) -> Result<(), Error> {
    for i in 0..config.screen_height {
        if i == config.screen_width / 3 {
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
    let stdin = stdin();
    let config = init_editor()?;

    enable_raw_mode()?;

    loop {
        refresh_screen(&config)?;
        match process_key_press(&stdin) {
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
