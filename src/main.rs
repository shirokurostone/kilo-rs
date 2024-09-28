use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{stdin, Error, Read};

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
    }
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

fn run() -> Result<(), Error> {
    let stdin = stdin();
    enable_raw_mode()?;

    loop {
        match process_key_press(&stdin) {
            Err(_) => break,
            Ok(_) => continue,
        }
    }

    disable_raw_mode()?;

    Ok(())
}
