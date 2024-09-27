use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::{stdin, Error, Read};

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
    }
}

fn run() -> Result<(), Error> {
    let mut buf = [0u8; 1];

    enable_raw_mode()?;

    loop {
        match stdin().read(&mut buf)? {
            0 => break,
            _ => match buf[0] as char {
                '\x11' => break, // ctrl+q
                c => {
                    print!("{}\r\n", c);
                }
            },
        }
    }

    disable_raw_mode()?;

    Ok(())
}
