use crate::TAB_STOP;
use std::fs::File;
use std::io::{BufRead, BufReader, Error};

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

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
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

    pub fn load_string(&mut self, text: String) {
        let mut lines: Vec<EditorLine> = Vec::new();

        for line in text.lines() {
            let render = self.convert_render(line);
            lines.push(EditorLine {
                line: line.to_string(),
                render,
            });
        }

        self.lines = lines;
        self.filepath = None;
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

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::EditorBuffer;

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
}
