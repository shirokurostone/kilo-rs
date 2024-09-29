use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Error, Read, Write};
use std::time::SystemTime;

const KILO_VERSION: &str = "0.1.0";
const TAB_STOP: usize = 8;

#[derive(Debug, PartialEq)]
struct EditorConfig {
    cx: usize,
    cy: usize,
    rx: usize,
    offset_x: usize,
    offset_y: usize,
    screen_width: usize,
    screen_height: usize,
    buffer: EditorBuffer,
    message: String,
    message_time: SystemTime,
}

#[derive(Debug, PartialEq)]
struct EditorLine {
    line: String,
    render: String,
}

#[derive(Debug, PartialEq)]
struct EditorBuffer {
    lines: Vec<EditorLine>,
    filepath: Option<String>,
}

impl EditorBuffer {
    fn new() -> EditorBuffer {
        EditorBuffer {
            lines: Vec::new(),
            filepath: None,
        }
    }

    fn len(&self) -> usize {
        self.lines.len()
    }

    fn get_line(&self, num: usize) -> Option<String> {
        self.lines.get(num).map(|el| el.line.clone())
    }

    fn get_render(&self, num: usize) -> Option<String> {
        self.lines.get(num).map(|el| el.render.clone())
    }

    fn get_filepath(&self) -> Option<String> {
        self.filepath.clone()
    }

    fn load_file(&mut self, path: String) -> Result<(), Error>{
        let mut lines: Vec<EditorLine> = Vec::new();

        let file = File::open(&path)?;
        let file_reader = BufReader::new(file);
        for ret in file_reader.lines() {
            let line = ret?;

            let render = convert_render(&line);
            lines.push(EditorLine { line, render });
        }

        self.lines = lines;
        self.filepath = Some(path.clone());

        Ok(())
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Err(err) = run(args) {
        eprintln!("{}", err);
    }
}

fn init_editor() -> Result<EditorConfig, Error> {
    let size = crossterm::terminal::size()?;

    Ok(EditorConfig {
        cx: 0,
        cy: 0,
        rx: 0,
        offset_x: 0,
        offset_y: 0,
        screen_width: size.0 as usize,
        screen_height: size.1 as usize - 2,
        buffer: EditorBuffer::new(),
        message: "HELP: Ctrl+Q = quit".to_string(),
        message_time: SystemTime::now(),
    })
}

fn convert_render(line: &str) -> String {
    let mut render = String::new();
    let mut i = 0;
    for c in line.chars() {
        match c {
            '\t' => {
                render.push(' ');
                i += 1;
                while i % TAB_STOP != 0 {
                    render.push(' ');
                    i += 1;
                }
            }
            c => {
                render.push(c);
            }
        }
        i += 1;
    }

    render
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
        c => Ok(EditorKey::OtherKey(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        convert_render, editor_cx_to_rx, read_editor_key, scroll, EditorBuffer, EditorConfig,
        EditorKey, EditorLine,
    };
    use std::io::BufReader;
    use std::time::SystemTime;

    #[test]
    fn test_convert_render() {
        assert_eq!("hoge", convert_render("hoge"));

        assert_eq!("        ", convert_render("\t"));
        assert_eq!("1       ", convert_render("1\t"));
        assert_eq!("12      ", convert_render("12\t"));
        assert_eq!("123     ", convert_render("123\t"));
        assert_eq!("1234    ", convert_render("1234\t"));
        assert_eq!("12345   ", convert_render("12345\t"));
        assert_eq!("123456  ", convert_render("123456\t"));
        assert_eq!("1234567 ", convert_render("1234567\t"));
        assert_eq!("12345678        ", convert_render("12345678\t"));
    }

    #[test]
    fn test_editor_cx_to_rx() {
        let config = EditorConfig {
            cx: 4,
            cy: 0,
            rx: 0,
            offset_x: 0,
            offset_y: 0,
            screen_width: 0,
            screen_height: 0,
            buffer: EditorBuffer {
                lines: vec![EditorLine {
                    line: "123\t456".to_string(),
                    render: "123     456".to_string(),
                }],
                filepath: None,
            },
            message: "".to_string(),
            message_time: SystemTime::now(),
        };

        let rx = editor_cx_to_rx(&config);
        assert_eq!(8, rx);
    }

    #[test]
    fn test_scroll() {
        let mut config = EditorConfig {
            cx: 0,
            cy: 0,
            rx: 0,
            offset_x: 0,
            offset_y: 0,
            screen_width: 20,
            screen_height: 20,
            buffer: EditorBuffer::new(),
            message: "".to_string(),
            message_time: SystemTime::now(),
        };

        for _ in 0..100 {
            config.buffer.lines.push(EditorLine {
                line: "*".to_string().repeat(100),
                render: "*".to_string().repeat(100),
            })
        }

        config.cx = 10;
        config.cy = 0;
        config.rx = 0;
        config.offset_x = 0;
        config.offset_y = 0;
        scroll(&mut config);
        assert_eq!(10, config.rx);
        assert_eq!(0, config.offset_x);

        config.cx = 10;
        config.cy = 0;
        config.rx = 0;
        config.offset_x = 50;
        config.offset_y = 0;
        scroll(&mut config);
        assert_eq!(10, config.rx);
        assert_eq!(10, config.offset_x);

        config.cx = 50;
        config.cy = 0;
        config.rx = 0;
        config.offset_x = 0;
        config.offset_y = 0;
        scroll(&mut config);
        assert_eq!(50, config.rx);
        assert_eq!(31, config.offset_x);

        config.cx = 0;
        config.cy = 10;
        config.rx = 0;
        config.offset_x = 0;
        config.offset_y = 50;
        scroll(&mut config);
        assert_eq!(10, config.offset_y);

        config.cx = 0;
        config.cy = 50;
        config.rx = 0;
        config.offset_x = 0;
        config.offset_y = 0;
        scroll(&mut config);
        assert_eq!(31, config.offset_y);
    }

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

fn process_key_press(config: &mut EditorConfig, editor_key: EditorKey) -> Result<(), Error> {
    let current_line = config.buffer.get_line(config.cy);

    match editor_key {
        EditorKey::Exit => {
            // ctrl+q
            return Err(Error::other("exit"));
        }
        EditorKey::ArrowDown => {
            if config.buffer.len() != 0 && config.cy < config.buffer.len() {
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
            } else if config.cy > 0 {
                if let Some(line) = config.buffer.get_line(config.cy - 1) {
                    config.cy -= 1;
                    config.cx = line.len();
                }
            }
        }
        EditorKey::ArrowRight => {
            if let Some(line) = current_line {
                if config.cx < line.len() {
                    config.cx += 1;
                } else if config.cx == line.len() {
                    config.cy += 1;
                    config.cx = 0;
                }
            }
        }
        EditorKey::PageUp => {
            config.cy = config.offset_y;
            for _ in 0..config.screen_height {
                if config.cy > 0 {
                    config.cy -= 1;
                }
            }
        }
        EditorKey::PageDown => {
            config.cy = config.offset_y + config.screen_height - 1;
            for _ in 0..config.screen_height {
                if config.buffer.len() != 0 && config.cy < config.buffer.len() {
                    config.cy += 1;
                }
            }
        }
        EditorKey::Home => {
            config.cx = 0;
        }
        EditorKey::End => {
            if config.cy < config.buffer.len() {
                if let Some(line) = config.buffer.get_line(config.cy) {
                    config.cx = line.len();
                }
            }
        }
        EditorKey::Delete => {}
        EditorKey::OtherKey(c) => {
            print!("{}\r\n", c);
        }
    }

    let new_line = config.buffer.get_line(config.cy);
    if let Some(line) = new_line {
        if line.len() < config.cx {
            config.cx = line.len();
        }
    }

    Ok(())
}

fn editor_cx_to_rx(config: &EditorConfig) -> usize {
    let mut rx = 0;
    if let Some(line) = config.buffer.get_line(config.cy) {
        for c in line.chars().take(config.cx) {
            if c == '\t' {
                rx += (TAB_STOP - 1) - (rx % TAB_STOP);
            }
            rx += 1;
        }
    }
    rx
}

fn scroll(config: &mut EditorConfig) {
    config.rx = 0;

    if config.cy < config.buffer.len() {
        config.rx = editor_cx_to_rx(config)
    }

    if config.rx < config.offset_x {
        config.offset_x = config.rx;
    }
    if config.rx >= config.offset_x + config.screen_width {
        config.offset_x = config.rx - config.screen_width + 1;
    }

    if config.cy < config.offset_y {
        config.offset_y = config.cy;
    }
    if config.cy >= config.offset_y + config.screen_height {
        config.offset_y = config.cy - config.screen_height + 1;
    }
}

fn refresh_screen(config: &mut EditorConfig) -> Result<(), Error> {
    let mut buf = String::new();

    scroll(config);

    buf.push_str("\x1b[?25l");
    buf.push_str("\x1b[H");

    draw_rows(config, &mut buf)?;
    draw_statusbar(config, &mut buf)?;
    draw_messagebar(config, &mut buf)?;

    let cursor = format!(
        "\x1b[{};{}H",
        (config.cy - config.offset_y) + 1,
        (config.rx - config.offset_x) + 1
    );
    buf.push_str(&cursor);

    buf.push_str("\x1b[?25h");

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

fn draw_rows(config: &EditorConfig, buf: &mut String) -> Result<(), Error> {
    for i in 0..config.screen_height {
        let file_line_no = i + config.offset_y;

        if file_line_no < config.buffer.len() {
            if let Some(render) = config.buffer.get_render(file_line_no) {
                let l: String = render
                    .chars()
                    .skip(config.offset_x)
                    .take(config.screen_width)
                    .collect();
                buf.push_str(&l);
            }
        } else if config.buffer.len() == 0 && i == config.screen_height / 3 {
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
        buf.push_str("\r\n");
    }

    Ok(())
}

fn draw_statusbar(config: &EditorConfig, buf: &mut String) -> Result<(), Error> {
    buf.push_str("\x1b[7m");

    let status = format!(
        "{:<20} - {} lines",
        config.buffer.get_filepath().unwrap_or_else(|| "[No Name]".to_string()),
        config.screen_height,
    );

    if config.screen_width < status.len() {
        let s: String = status.chars().take(config.screen_width).collect();
        buf.push_str(&s);
    } else {
        buf.push_str(&status);

        let right_status = format!("{}/{}", config.cy + 1, config.buffer.len());
        if config.screen_width as isize - status.len() as isize - right_status.len() as isize > 0 {
            for _ in 0..(config.screen_width - status.len() - right_status.len()) {
                buf.push(' ');
            }
            buf.push_str(&right_status);
        } else {
            for _ in 0..(config.screen_width - status.len()) {
                buf.push(' ');
            }
        }
    }

    buf.push_str("\x1b[m");
    buf.push_str("\r\n");

    Ok(())
}

fn draw_messagebar(config: &EditorConfig, buf: &mut String) -> Result<(), Error> {
    buf.push_str("\x1b[K");

    let now = SystemTime::now();

    if let Ok(t) = now.duration_since(config.message_time) {
        if t.as_secs() < 5 {
            buf.push_str(&config.message);
        }
    }

    Ok(())
}

fn run(args: Vec<String>) -> Result<(), Error> {
    let mut stdin = stdin();
    let mut config = init_editor()?;

    if args.len() > 1 {
        config.buffer.load_file(args.get(1).unwrap().to_string())?;
    }

    enable_raw_mode()?;

    loop {
        refresh_screen(&mut config)?;
        match process_key_press(&mut config, read_editor_key(&mut stdin)?) {
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
