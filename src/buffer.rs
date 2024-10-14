use crate::TAB_STOP;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, Write};
use std::os::unix::fs::MetadataExt;

#[derive(Debug, PartialEq, Copy, Clone)]
enum HighlightType {
    Number,
    String,
    Comment,
    MultilineComment,
    Keyword1,
    Keyword2,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum FileType {
    C,
}
impl FileType {
    fn select_file_type(filepath: &str) -> Option<FileType> {
        let file_types = [FileType::C];

        for ft in file_types {
            for extension in ft.extension() {
                if filepath.ends_with(extension) {
                    return Some(ft);
                }
            }
        }

        None
    }

    fn extension(&self) -> Vec<&'static str> {
        match self {
            FileType::C => vec![".c", ".h", ".cpp"],
        }
    }

    fn keyword1(&self) -> Vec<&'static str> {
        match self {
            FileType::C => vec![
                "switch", "if", "while", "for", "break", "continue", "return", "else", "struct",
                "union", "typedef", "static", "enum", "class", "case",
            ],
        }
    }

    fn keyword2(&self) -> Vec<&'static str> {
        match self {
            FileType::C => vec![
                "int", "long", "double", "float", "char", "unsigned", "signed", "void",
            ],
        }
    }

    fn is_highlight(&self, highlight_type: HighlightType) -> bool {
        match self {
            FileType::C => match highlight_type {
                HighlightType::Number => true,
                HighlightType::String => true,
                HighlightType::Comment => true,
                HighlightType::MultilineComment => true,
                HighlightType::Keyword1 => true,
                HighlightType::Keyword2 => true,
            },
        }
    }

    fn singleline_comment_start(&self) -> Option<&'static str> {
        match self {
            FileType::C => Some("//"),
        }
    }

    fn multiline_comment_start(&self) -> Option<&'static str> {
        match self {
            FileType::C => Some("/*"),
        }
    }

    fn multiline_comment_end(&self) -> Option<&'static str> {
        match self {
            FileType::C => Some("*/"),
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            FileType::C => "C",
        }
    }
}

fn is_separator(c: char) -> bool {
    c == ' '
        || c == '\t'
        || c == '\r'
        || c == '\n'
        || c == '\0'
        || c == ','
        || c == '.'
        || c == '('
        || c == ')'
        || c == '+'
        || c == '-'
        || c == '/'
        || c == '*'
        || c == '='
        || c == '%'
        || c == '<'
        || c == '>'
        || c == '['
        || c == ']'
}

#[derive(Debug, PartialEq)]
struct EditorLine {
    raw: String,
    render: String,
    highlight: Vec<Highlight>,
    file_type: Option<FileType>,
    open_comment: bool,
}

impl EditorLine {
    fn new(line: String, file_type: Option<FileType>) -> EditorLine {
        let mut el = EditorLine {
            raw: line,
            render: String::new(),
            highlight: Vec::new(),
            file_type,
            open_comment: false,
        };

        el.render = el.convert_render(&el.raw);
        el.clear_highlight(false);
        el
    }

    fn remove_char(&mut self, index: usize) {
        self.raw.remove(index);
        self.render = self.convert_render(&self.raw);
    }

    fn insert_char(&mut self, index: usize, c: char) {
        self.raw.insert(index, c);
        self.render = self.convert_render(&self.raw);
    }

    fn insert_str(&mut self, index: usize, str: &str) {
        self.raw.insert_str(index, str);
        self.render = self.convert_render(&self.raw);
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

    pub fn clear_highlight(&mut self, open_comment: bool) -> bool {
        if self.render.len() != self.highlight.len() {
            self.highlight.resize(self.render.len(), Highlight::Normal);
        }

        let mut prev_highlight = Highlight::Normal;
        let mut prev_separator = true;
        let mut prev_char = '\0';
        let mut in_string = false;
        let mut in_comment = open_comment;
        let mut quote = '\0';
        let mut i = 0;

        let keyword_func = |render: &String,
                            highlight: &mut Vec<Highlight>,
                            keywords: Vec<&'static str>,
                            i: &mut usize,
                            prev_highlight: &mut Highlight,
                            keyword_highlight: Highlight|
         -> bool {
            for keyword in keywords {
                let s: String = render.chars().skip(*i).take(keyword.len()).collect();
                if keyword == s {
                    if *i + keyword.len() == render.len() {
                        for j in *i..*i + keyword.len() {
                            highlight[j] = keyword_highlight;
                        }
                        *i += keyword.len();
                        *prev_highlight = keyword_highlight;
                        return true;
                    } else if *i + keyword.len() + 1 < render.len() {
                        if let Some(end) = render.chars().nth(*i + keyword.len() + 1) {
                            if is_separator(end) {
                                for j in *i..*i + keyword.len() {
                                    highlight[j] = keyword_highlight;
                                }
                                *i += keyword.len();
                                *prev_highlight = keyword_highlight;
                                return true;
                            }
                        }
                    }
                }
            }
            false
        };

        'char_loop: while i < self.render.len() {
            if let Some(c) = self.render.chars().nth(i) {
                self.highlight[i] = Highlight::Normal;
                if let Some(file_type) = self.file_type {
                    if file_type.is_highlight(HighlightType::Number) {
                        if c.is_ascii_digit()
                            && (prev_separator || prev_highlight == Highlight::Number)
                        {
                            self.highlight[i] = Highlight::Number;
                            prev_separator = false;
                        } else if c == '.' && prev_highlight == Highlight::Number {
                            self.highlight[i] = Highlight::Number;
                            prev_separator = false;
                        }
                    }
                    if file_type.is_highlight(HighlightType::String) {
                        if in_string {
                            self.highlight[i] = Highlight::String;
                            if c == quote && prev_char != '\\' {
                                in_string = false;
                            }
                            prev_separator = true;
                        } else {
                            if c == '\'' || c == '"' {
                                in_string = true;
                                quote = c;
                                self.highlight[i] = Highlight::String;
                            }
                        }
                    }
                    if file_type.is_highlight(HighlightType::Comment) {
                        if !in_string && !in_comment {
                            if let Some(comment_start) = file_type.singleline_comment_start() {
                                let s: String = self
                                    .render
                                    .chars()
                                    .skip(i)
                                    .take(comment_start.len())
                                    .collect();
                                if comment_start == s {
                                    for j in i..self.render.len() {
                                        self.highlight[j] = Highlight::Comment;
                                    }
                                    self.open_comment = false;
                                    return false;
                                }
                            }
                        }
                    }

                    if file_type.is_highlight(HighlightType::MultilineComment) {
                        if !in_comment {
                            if let Some(comment_start) = file_type.multiline_comment_start() {
                                let s: String = self
                                    .render
                                    .chars()
                                    .skip(i)
                                    .take(comment_start.len())
                                    .collect();
                                if comment_start == s {
                                    in_comment = true;
                                    for j in i..i + comment_start.len() {
                                        self.highlight[j] = Highlight::MultilineComment;
                                    }
                                    i += comment_start.len();
                                    continue 'char_loop;
                                }
                            }
                        } else {
                            self.highlight[i] = Highlight::MultilineComment;
                            if let Some(comment_end) = file_type.multiline_comment_end() {
                                let s: String = self
                                    .render
                                    .chars()
                                    .skip(i)
                                    .take(comment_end.len())
                                    .collect();
                                if comment_end == s {
                                    in_comment = false;
                                    prev_separator = true;
                                    for j in i..i + comment_end.len() {
                                        self.highlight[j] = Highlight::MultilineComment;
                                    }
                                    i += comment_end.len();
                                    continue 'char_loop;
                                }
                            }
                        }
                    }

                    if file_type.is_highlight(HighlightType::Keyword1) {
                        if prev_separator && !in_comment {
                            if keyword_func(
                                &self.render,
                                &mut self.highlight,
                                file_type.keyword1(),
                                &mut i,
                                &mut prev_highlight,
                                Highlight::Keyword1,
                            ) {
                                continue 'char_loop;
                            }
                        }
                    }

                    if file_type.is_highlight(HighlightType::Keyword2) {
                        if prev_separator && !in_comment {
                            if keyword_func(
                                &self.render,
                                &mut self.highlight,
                                file_type.keyword2(),
                                &mut i,
                                &mut prev_highlight,
                                Highlight::Keyword2,
                            ) {
                                continue 'char_loop;
                            }
                        }
                    }
                }
                prev_separator = is_separator(c);
                prev_highlight = self.highlight[i];
                prev_char = c;
            }
            i += 1;
        }

        self.open_comment = in_comment;
        in_comment
    }

    fn highlight(&mut self, begin: usize, end: usize, highlight: Highlight) {
        for i in begin..end {
            self.highlight[i] = highlight;
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Highlight {
    Normal,
    Number,
    Match,
    String,
    Comment,
    MultilineComment,
    Keyword1,
    Keyword2,
}

impl Highlight {
    fn color(&self) -> usize {
        match self {
            Highlight::Normal => 37,
            Highlight::Number => 31,
            Highlight::Match => 34,
            Highlight::String => 35,
            Highlight::Comment => 36,
            Highlight::MultilineComment => 36,
            Highlight::Keyword1 => 33,
            Highlight::Keyword2 => 32,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct EditorBuffer {
    lines: Vec<EditorLine>,
    filepath: Option<String>,
    dirty: bool,
    file_type: Option<FileType>,
}

impl EditorBuffer {
    pub fn new() -> EditorBuffer {
        EditorBuffer {
            lines: Vec::new(),
            filepath: None,
            dirty: false,
            file_type: None,
        }
    }

    pub fn get_file_type(&self) -> Option<FileType> {
        self.file_type
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn get_line(&self, num: usize) -> Option<String> {
        self.lines.get(num).map(|el| el.raw.clone())
    }

    pub fn clear_highlight(&mut self, cy: usize) {
        let mut open_comment = if cy == 0 {
            false
        } else {
            self.lines[cy - 1].open_comment
        };

        for i in cy..self.lines.len() {
            open_comment = self.lines[i].clear_highlight(open_comment);
        }
    }

    pub fn highlight(&mut self, cx: usize, cy: usize, width: usize, highlight: Highlight) {
        let begin = self.cx_to_rx(cx, cy);
        let end = self.cx_to_rx(cx + width, cy);
        self.lines[cy].highlight(begin, end, highlight);
    }

    pub fn get_render(&self, num: usize, offset: usize, width: usize) -> Option<String> {
        self.lines.get(num).map(|el| {
            let mut output = String::new();
            let mut current_color = Highlight::Normal;

            el.render
                .chars()
                .enumerate()
                .skip(offset)
                .take(width)
                .for_each(|(i, c)| {
                    if c.is_ascii_control() {
                        output.push_str("\x1b[7m");
                        match c {
                            '\x00' => output.push('@'),
                            '\x01'..='\x1a' => output.push(((c as u8) + b'@') as char),
                            _ => output.push('?'),
                        }
                        output.push_str("\x1b[m");

                        let s = format!("\x1b[{}m", current_color.color());
                        output.push_str(&s);
                    } else {
                        match el.highlight[i] {
                            Highlight::Normal => {
                                if current_color != Highlight::Normal {
                                    output.push_str("\x1b[39m");
                                    current_color = Highlight::Normal;
                                }
                                output.push(c);
                            }
                            hi => {
                                if current_color != hi {
                                    let s = format!("\x1b[{}m", hi.color());
                                    output.push_str(&s);
                                    current_color = hi;
                                }
                                output.push(c);
                            }
                        }
                    }
                });
            output.push_str("\x1b[39m");
            output
        })
    }

    pub fn get_filepath(&self) -> Option<String> {
        self.filepath.clone()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn load_file(&mut self, path: String) -> Result<(), Error> {
        let mut lines: Vec<EditorLine> = Vec::new();

        let file = File::open(&path)?;
        let file_reader = BufReader::new(file);
        self.file_type = FileType::select_file_type(&path);
        for ret in file_reader.lines() {
            let el = EditorLine::new(ret?, self.file_type);
            lines.push(el);
        }

        self.lines = lines;
        self.filepath = Some(path.clone());
        self.dirty = false;
        self.clear_highlight(0);

        Ok(())
    }

    pub fn save_file(&mut self, path: String) -> Result<u64, Error> {
        let mut file = File::create(&path)?;
        file.write_all(
            self.lines
                .iter()
                .map(|el| el.raw.clone())
                .collect::<Vec<String>>()
                .join("\n")
                .as_bytes(),
        )?;
        file.flush()?;
        self.filepath = Some(path.clone());
        self.file_type = FileType::select_file_type(&path);
        for line in &mut self.lines {
            line.file_type = self.file_type;
        }
        self.dirty = false;
        self.clear_highlight(0);

        Ok(file.metadata()?.size())
    }

    pub fn overwrite_file(&mut self) -> Result<u64, Error> {
        if let Some(path) = &self.filepath {
            self.save_file(path.clone())
        } else {
            Err(Error::other("no save file path"))
        }
    }

    pub fn load_string(&mut self, text: String) {
        let mut lines: Vec<EditorLine> = Vec::new();

        for line in text.lines() {
            lines.push(EditorLine::new(line.to_string(), None));
        }

        self.lines = lines;
        self.filepath = None;
        self.file_type = None;
        for line in &mut self.lines {
            line.file_type = self.file_type;
        }
        self.dirty = false;
        self.clear_highlight(0);
    }

    pub fn insert_line(&mut self, cy: usize, line: String) {
        self.lines
            .insert(cy, EditorLine::new(line.to_string(), self.file_type));
        self.dirty = true;
    }

    pub fn insert_char(&mut self, cx: usize, cy: usize, c: char) {
        if let Some(el) = self.lines.get_mut(cy) {
            el.insert_char(cx, c);
            self.dirty = true;
            self.clear_highlight(cy);
        }
    }

    pub fn delete_char(&mut self, cx: usize, cy: usize) {
        if let Some(el) = self.lines.get_mut(cy) {
            if cx < el.raw.len() {
                el.remove_char(cx);
                self.dirty = true;
                self.clear_highlight(cy);
            }
        }
    }

    pub fn delete_line(&mut self, cy: usize) {
        self.lines.remove(cy);
        self.dirty = true;
    }

    pub fn replace_line(&mut self, cy: usize, new_line: String) {
        self.lines[cy] = EditorLine::new(new_line, self.file_type);
    }

    pub fn append_string(&mut self, cx: usize, cy: usize, message: String) {
        if let Some(el) = self.lines.get_mut(cy) {
            el.insert_str(cx, &message);
            self.dirty = true;
            self.clear_highlight(cy);
        }
    }

    pub fn cx_to_rx(&self, cx: usize, cy: usize) -> usize {
        let mut rx = 0;
        if let Some(line) = self.get_line(cy) {
            for c in line.chars().take(cx) {
                if c == '\t' {
                    rx += (TAB_STOP - 1) - (rx % TAB_STOP);
                }
                rx += 1;
            }
        }
        rx
    }
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorBuffer, EditorLine};

    #[test]
    fn test_convert_render() {
        let el = EditorLine::new("".to_string(), None);

        assert_eq!("hoge", el.convert_render("hoge"));

        assert_eq!("        ", el.convert_render("\t"));
        assert_eq!("1       ", el.convert_render("1\t"));
        assert_eq!("12      ", el.convert_render("12\t"));
        assert_eq!("123     ", el.convert_render("123\t"));
        assert_eq!("1234    ", el.convert_render("1234\t"));
        assert_eq!("12345   ", el.convert_render("12345\t"));
        assert_eq!("123456  ", el.convert_render("123456\t"));
        assert_eq!("1234567 ", el.convert_render("1234567\t"));
        assert_eq!("12345678        ", el.convert_render("12345678\t"));
    }

    #[test]
    fn test_cx_to_rx() {
        let mut buffer = EditorBuffer::new();
        buffer.load_string("123\t456".to_string());

        let rx = buffer.cx_to_rx(4, 0);
        assert_eq!(8, rx);
    }
}
