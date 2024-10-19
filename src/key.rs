use std::io::{Error, Read};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EditorKey {
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

fn read_single_key(reader: &mut dyn Read) -> Result<char, Error> {
    let mut buf = [0u8; 1];

    loop {
        match reader.read(&mut buf)? {
            0 => continue,
            _ => return Ok(buf[0] as char),
        }
    }
}

pub fn read_editor_key(reader: &mut dyn Read) -> Result<EditorKey, Error> {
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
    use super::EditorKey;
    use crate::key::read_editor_key;
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
}
