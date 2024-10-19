mod buffer;
mod command;
mod key;
mod screen;

use crate::buffer::EditorBuffer;
use crate::command::{process_command, resolve_command};
use crate::key::read_key;
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
        let key = read_key(&mut stdin)?;
        let command = resolve_command(key);
        match process_command(
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
