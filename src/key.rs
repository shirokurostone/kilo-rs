use std::io::{Error, Read};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Key {
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

fn read_char(reader: &mut dyn Read) -> Result<char, Error> {
    let mut buf = [0u8; 1];

    loop {
        match reader.read(&mut buf)? {
            0 => continue,
            _ => return Ok(buf[0] as char),
        }
    }
}

pub fn read_key(reader: &mut dyn Read) -> Result<Key, Error> {
    let c = read_char(reader)?;
    let escape_sequence_table = [
        ("\x1b[A", Key::ArrowUp),
        ("\x1b[B", Key::ArrowDown),
        ("\x1b[C", Key::ArrowRight),
        ("\x1b[D", Key::ArrowLeft),
        ("\x1b[H", Key::Home),
        ("\x1b[F", Key::End),
        ("\x1b[1~", Key::Home),
        ("\x1b[3~", Key::Delete),
        ("\x1b[4~", Key::End),
        ("\x1b[5~", Key::PageUp),
        ("\x1b[6~", Key::PageDown),
        ("\x1b[7~", Key::Home),
        ("\x1b[8~", Key::End),
        ("\x1bOH", Key::Home),
        ("\x1bOF", Key::End),
    ];

    match c {
        '\r' => Ok(Key::Enter),
        '\x01'..'\x1b' => Ok(Key::ControlSequence(((c as u8) + b'a' - 1) as char)),
        '\x1b' => {
            let mut buf = String::from("\x1b");
            loop {
                let c2 = read_char(reader)?;
                buf.push(c2);

                let matches = escape_sequence_table
                    .iter()
                    .filter(|seq| seq.0.starts_with(&buf))
                    .collect::<Vec<_>>();

                if matches.is_empty() {
                    return Ok(Key::Escape);
                } else if matches.len() == 1 && buf.eq(matches[0].0) {
                    return Ok(matches[0].1);
                }
            }
        }
        '\x7f' => Ok(Key::Backspace),
        c => Ok(Key::NormalKey(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::Key;
    use crate::key::read_key;
    use std::io::BufReader;

    fn assert_read_editor_key(input: &str, expected: Key) {
        let data = input.bytes().collect::<Vec<u8>>();
        let mut reader = BufReader::new(&data[..]);
        let actual = read_key(&mut reader);
        assert_eq!(expected, actual.unwrap(), "input:{}", input.escape_debug());
    }

    #[test]
    fn test_read_editor_key_escape() {
        assert_read_editor_key("\x1b[A", Key::ArrowUp);
        assert_read_editor_key("\x1b[B", Key::ArrowDown);
        assert_read_editor_key("\x1b[C", Key::ArrowRight);
        assert_read_editor_key("\x1b[D", Key::ArrowLeft);
        assert_read_editor_key("\x1b[H", Key::Home);
        assert_read_editor_key("\x1b[F", Key::End);

        assert_read_editor_key("\x1b[1~", Key::Home);
        assert_read_editor_key("\x1b[3~", Key::Delete);
        assert_read_editor_key("\x1b[4~", Key::End);
        assert_read_editor_key("\x1b[5~", Key::PageUp);
        assert_read_editor_key("\x1b[6~", Key::PageDown);
        assert_read_editor_key("\x1b[7~", Key::Home);
        assert_read_editor_key("\x1b[8~", Key::End);

        assert_read_editor_key("\x1bOH", Key::Home);
        assert_read_editor_key("\x1bOF", Key::End);
    }

    #[test]
    fn test_read_editor_key() {
        assert_read_editor_key("\r", Key::Enter);
        assert_read_editor_key("\x7f", Key::Backspace);
        assert_read_editor_key(" ", Key::NormalKey(' '));
        assert_read_editor_key("~", Key::NormalKey('~'));
        assert_read_editor_key("\x01", Key::ControlSequence('a'));
        assert_read_editor_key("\x1a", Key::ControlSequence('z'));
    }
}
