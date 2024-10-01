use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Error, Read, Write};
use std::time::SystemTime;

const KILO_VERSION: &str = "0.1.0";
const TAB_STOP: usize = 8;

#[derive(Debug, PartialEq)]
struct EditorConfig {
    screen: EditorScreen,
    buffer: EditorBuffer,
    message_bar: MessageBar,
}

#[derive(Debug, PartialEq)]
pub struct EditorScreen {
    cx: usize,
    cy: usize,
    rx: usize,
    offset_x: usize,
    offset_y: usize,
    width: usize,
    height: usize,
}

impl EditorScreen {
    pub fn new() -> EditorScreen {
        EditorScreen {
            cx: 0,
            cy: 0,
            rx: 0,
            offset_x: 0,
            offset_y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn down(&mut self, buffer: &EditorBuffer) {
        if buffer.len() != 0 && self.cy < buffer.len() {
            self.cy += 1;
        }
    }

    pub fn up(&mut self, _: &EditorBuffer) {
        if self.cy > 0 {
            self.cy -= 1;
        }
    }

    pub fn left(&mut self, buffer: &EditorBuffer) {
        if self.cx > 0 {
            self.cx -= 1;
        } else if self.cy > 0 {
            if let Some(line) = buffer.get_line(self.cy - 1) {
                self.cy -= 1;
                self.cx = line.len();
            }
        }
    }

    pub fn right(&mut self, buffer: &EditorBuffer) {
        if let Some(line) = buffer.get_line(self.cy) {
            if self.cx < line.len() {
                self.cx += 1;
            } else if self.cx == line.len() {
                self.cy += 1;
                self.cx = 0;
            }
        }
    }

    pub fn page_up(&mut self, buffer: &EditorBuffer) {
        self.cy = self.offset_y;
        for _ in 0..self.height {
            self.up(buffer);
        }
    }

    pub fn page_down(&mut self, buffer: &EditorBuffer) {
        self.cy = self.offset_y + self.height - 1;
        for _ in 0..self.height {
            self.down(buffer);
        }
    }

    pub fn home(&mut self, _: &EditorBuffer) {
        self.cx = 0;
    }

    pub fn end(&mut self, buffer: &EditorBuffer) {
        if self.cy < buffer.len() {
            if let Some(line) = buffer.get_line(self.cy) {
                self.cx = line.len();
            }
        }
    }

    pub fn adjust(&mut self, buffer: &EditorBuffer) {
        self.rx = 0;

        if let Some(line) = buffer.get_line(self.cy) {
            if line.len() < self.cx {
                self.cx = line.len();
            }
        }

        if self.cy < buffer.len() {
            self.rx = self.cx_to_rx(buffer);
        }

        if self.rx < self.offset_x {
            self.offset_x = self.rx;
        }
        if self.rx >= self.offset_x + self.width {
            self.offset_x = self.rx - self.width + 1;
        }

        if self.cy < self.offset_y {
            self.offset_y = self.cy;
        }
        if self.cy >= self.offset_y + self.height {
            self.offset_y = self.cy - self.height + 1;
        }
    }

    fn cx_to_rx(&self, buffer: &EditorBuffer) -> usize {
        let mut rx = 0;
        if let Some(line) = buffer.get_line(self.cy) {
            for c in line.chars().take(self.cx) {
                if c == '\t' {
                    rx += (TAB_STOP - 1) - (rx % TAB_STOP);
                }
                rx += 1;
            }
        }
        rx
    }
}

#[derive(Debug, PartialEq)]
struct EditorLine {
    line: String,
    render: String,
}

#[derive(Debug, PartialEq)]
pub struct EditorBuffer {
    lines: Vec<EditorLine>,
    filepath: Option<String>,
}

impl EditorBuffer {
    pub fn new() -> EditorBuffer {
        EditorBuffer {
            lines: Vec::new(),
            filepath: None,
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn get_line(&self, num: usize) -> Option<String> {
        self.lines.get(num).map(|el| el.line.clone())
    }

    pub fn get_render(&self, num: usize) -> Option<String> {
        self.lines.get(num).map(|el| el.render.clone())
    }

    pub fn get_filepath(&self) -> Option<String> {
        self.filepath.clone()
    }

    pub fn load_file(&mut self, path: String) -> Result<(), Error> {
        let mut lines: Vec<EditorLine> = Vec::new();

        let file = File::open(&path)?;
        let file_reader = BufReader::new(file);
        for ret in file_reader.lines() {
            let line = ret?;

            let render = self.convert_render(&line);
            lines.push(EditorLine { line, render });
        }

        self.lines = lines;
        self.filepath = Some(path.clone());

        Ok(())
    }

    fn convert_render(&self, line: &str) -> String {
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
}

#[derive(Debug, PartialEq)]
struct MessageBar {
    message: String,
    updated_at: SystemTime,
}

impl MessageBar {
    fn new(message: String, time: SystemTime) -> MessageBar {
        MessageBar {
            message,
            updated_at: time,
        }
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

    let mut screen = EditorScreen::new();
    screen.width = size.0 as usize;
    screen.height = size.1 as usize - 2;

    Ok(EditorConfig {
        screen,
        buffer: EditorBuffer::new(),
        message_bar: MessageBar::new("HELP: Ctrl+Q = quit".to_string(), SystemTime::now()),
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
    use super::{read_editor_key, EditorBuffer, EditorKey, EditorLine, EditorScreen};
    use std::io::BufReader;

    #[test]
    fn test_convert_render() {
        let buffer = EditorBuffer::new();

        assert_eq!("hoge", buffer.convert_render("hoge"));

        assert_eq!("        ", buffer.convert_render("\t"));
        assert_eq!("1       ", buffer.convert_render("1\t"));
        assert_eq!("12      ", buffer.convert_render("12\t"));
        assert_eq!("123     ", buffer.convert_render("123\t"));
        assert_eq!("1234    ", buffer.convert_render("1234\t"));
        assert_eq!("12345   ", buffer.convert_render("12345\t"));
        assert_eq!("123456  ", buffer.convert_render("123456\t"));
        assert_eq!("1234567 ", buffer.convert_render("1234567\t"));
        assert_eq!("12345678        ", buffer.convert_render("12345678\t"));
    }

    #[test]
    fn test_cx_to_rx() {
        let mut screen = EditorScreen::new();
        screen.cx = 4;

        let mut buffer = EditorBuffer::new();
        buffer.lines.push(EditorLine {
            line: "123\t456".to_string(),
            render: "123     456".to_string(),
        });

        let rx = screen.cx_to_rx(&buffer);
        assert_eq!(8, rx);
    }

    #[test]
    fn test_adjust() {
        let mut buffer = EditorBuffer::new();

        for _ in 0..100 {
            buffer.lines.push(EditorLine {
                line: "*".to_string().repeat(100),
                render: "*".to_string().repeat(100),
            })
        }

        let mut screen = EditorScreen::new();
        screen.width = 20;
        screen.height = 20;

        screen.cx = 200;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(100, screen.cx);
        assert_eq!(100, screen.rx);
        assert_eq!(81, screen.offset_x);

        screen.cx = 10;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(10, screen.rx);
        assert_eq!(0, screen.offset_x);

        screen.cx = 10;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 50;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(10, screen.rx);
        assert_eq!(10, screen.offset_x);

        screen.cx = 50;
        screen.cy = 0;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(50, screen.rx);
        assert_eq!(31, screen.offset_x);

        screen.cx = 0;
        screen.cy = 10;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 50;
        screen.adjust(&buffer);
        assert_eq!(10, screen.offset_y);

        screen.cx = 0;
        screen.cy = 50;
        screen.rx = 0;
        screen.offset_x = 0;
        screen.offset_y = 0;
        screen.adjust(&buffer);
        assert_eq!(31, screen.offset_y);
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

fn process_key_press(
    screen: &mut EditorScreen,
    buffer: &EditorBuffer,
    editor_key: EditorKey,
) -> Result<(), Error> {
    match editor_key {
        EditorKey::Exit => {
            // ctrl+q
            return Err(Error::other("exit"));
        }
        EditorKey::ArrowDown => screen.down(buffer),
        EditorKey::ArrowUp => screen.up(buffer),
        EditorKey::ArrowLeft => screen.left(buffer),
        EditorKey::ArrowRight => screen.right(buffer),
        EditorKey::PageUp => screen.page_up(buffer),
        EditorKey::PageDown => screen.page_down(buffer),
        EditorKey::Home => screen.home(buffer),
        EditorKey::End => screen.end(buffer),
        EditorKey::Delete => {}
        EditorKey::OtherKey(c) => {
            print!("{}\r\n", c);
        }
    }

    screen.adjust(buffer);

    Ok(())
}

fn refresh_screen(
    screen: &EditorScreen,
    buffer: &EditorBuffer,
    message_bar: &MessageBar,
) -> Result<(), Error> {
    let mut buf = String::new();

    buf.push_str("\x1b[?25l");
    buf.push_str("\x1b[H");

    draw_rows(screen, buffer, &mut buf)?;
    draw_statusbar(screen, buffer, &mut buf)?;
    draw_messagebar(message_bar, &mut buf)?;

    let cursor = format!(
        "\x1b[{};{}H",
        (screen.cy - screen.offset_y) + 1,
        (screen.rx - screen.offset_x) + 1
    );
    buf.push_str(&cursor);

    buf.push_str("\x1b[?25h");

    print!("{}", buf);
    stdout().flush()?;

    Ok(())
}

fn draw_rows(screen: &EditorScreen, buffer: &EditorBuffer, buf: &mut String) -> Result<(), Error> {
    for i in 0..screen.height {
        let file_line_no = i + screen.offset_y;

        if file_line_no < buffer.len() {
            if let Some(render) = buffer.get_render(file_line_no) {
                let l: String = render
                    .chars()
                    .skip(screen.offset_x)
                    .take(screen.width)
                    .collect();
                buf.push_str(&l);
            }
        } else if buffer.len() == 0 && i == screen.height / 3 {
            let title = format!("kilo-rs -- version {}", KILO_VERSION);
            let t: String = title.chars().take(screen.width).collect();
            let mut padding = (screen.width - t.len()) / 2;
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

fn draw_statusbar(
    screen: &EditorScreen,
    buffer: &EditorBuffer,
    buf: &mut String,
) -> Result<(), Error> {
    buf.push_str("\x1b[7m");

    let status = format!(
        "{:<20} - {} lines",
        buffer
            .get_filepath()
            .unwrap_or_else(|| "[No Name]".to_string()),
        screen.height,
    );

    if screen.width < status.len() {
        let s: String = status.chars().take(screen.width).collect();
        buf.push_str(&s);
    } else {
        buf.push_str(&status);

        let right_status = format!("{}/{}", screen.cy + 1, buffer.len());
        if screen.width as isize - status.len() as isize - right_status.len() as isize > 0 {
            for _ in 0..(screen.width - status.len() - right_status.len()) {
                buf.push(' ');
            }
            buf.push_str(&right_status);
        } else {
            for _ in 0..(screen.width - status.len()) {
                buf.push(' ');
            }
        }
    }

    buf.push_str("\x1b[m");
    buf.push_str("\r\n");

    Ok(())
}

fn draw_messagebar(message_bar: &MessageBar, buf: &mut String) -> Result<(), Error> {
    buf.push_str("\x1b[K");

    let now = SystemTime::now();

    if let Ok(t) = now.duration_since(message_bar.updated_at) {
        if t.as_secs() < 5 {
            buf.push_str(&message_bar.message);
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
        refresh_screen(&config.screen, &config.buffer, &config.message_bar)?;
        match process_key_press(
            &mut config.screen,
            &config.buffer,
            read_editor_key(&mut stdin)?,
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
