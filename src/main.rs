mod buffer;
mod escape_sequence;
mod key;
mod message_bar;
mod pane;
mod screen;
mod status_bar;
mod ui;

use crate::escape_sequence::{
    ESCAPE_SEQUENCE_CLEAR_SCREEN, ESCAPE_SEQUENCE_MOVE_CURSOR_TO_FIRST_POSITION,
};
use crate::key::read_key;
use crate::pane::Pane;
use crate::screen::refresh_screen;
use crate::ui::Terminal;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{stdin, stdout, Error, Write};
use std::time::SystemTime;

const KILO_VERSION: &str = "0.1.0";
const TAB_STOP: usize = 8;
const QUIT_TIMES: usize = 3;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Err(err) = run(args) {
        eprintln!("{}", err);
    }
}

fn run(args: Vec<String>) -> Result<(), Error> {
    let mut stdin = stdin();
    let mut pane = Pane::new("HELP: Ctrl+Q = quit".to_string(), SystemTime::now());
    let mut terminal = Terminal::new()?;

    pane.set_size(0, 0, terminal.get_width(), terminal.get_height());

    if args.len() > 1 {
        pane.screen()
            .buffer()
            .load_file(args.get(1).unwrap().to_string())?;
    }

    enable_raw_mode()?;

    loop {
        if terminal.update()? {
            pane.set_size(0, 0, terminal.get_width(), terminal.get_height());
        }

        refresh_screen(&mut pane)?;
        let key = read_key(&mut stdin)?;
        let command = pane.resolve_command(key);
        match pane.process_command(&mut stdin, command) {
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
