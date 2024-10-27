mod buffer;
mod command;
mod escape_sequence;
mod key;
mod screen;

use crate::buffer::EditorBuffer;
use crate::command::{process_command, resolve_command};
use crate::escape_sequence::{
    ESCAPE_SEQUENCE_CLEAR_SCREEN, ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION,
};
use crate::key::read_key;
use crate::screen::{refresh_screen, EditorScreen, MessageBar, Terminal};
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
    let mut terminal = Terminal::new()?;

    config
        .screen
        .set_size(0, 0, terminal.get_width(), terminal.get_height() - 2);
    config
        .message_bar
        .set_size(0, terminal.get_height() - 1, terminal.get_width(), 1);

    if args.len() > 1 {
        config.buffer.load_file(args.get(1).unwrap().to_string())?;
    }

    enable_raw_mode()?;

    loop {
        if terminal.update()? {
            config
                .screen
                .set_size(0, 0, terminal.get_width(), terminal.get_height() - 2);
            config
                .message_bar
                .set_size(0, terminal.get_height() - 1, terminal.get_width() - 1, 1);
        }

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

    print!(
        "{}{}",
        ESCAPE_SEQUENCE_CLEAR_SCREEN, ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION
    );
    stdout().flush()?;
    disable_raw_mode()?;

    Ok(())
}
