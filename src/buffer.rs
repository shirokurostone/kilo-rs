use crate::TAB_STOP;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, Write};
use std::os::unix::fs::MetadataExt;

#[derive(Debug, PartialEq)]
struct EditorLine {
    raw: String,
    render: String,
}

impl EditorLine {
    fn new(line: String) -> EditorLine {
        let mut el = EditorLine {
            raw: line,
            render: String::new(),
        };

        el.render = el.convert_render(&el.raw);
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
}

#[derive(Debug, PartialEq)]
pub struct EditorBuffer {
    lines: Vec<EditorLine>,
    filepath: Option<String>,
    dirty: bool,
}

impl EditorBuffer {
    pub fn new() -> EditorBuffer {
        EditorBuffer {
            lines: Vec::new(),
            filepath: None,
            dirty: false,
        }
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

    pub fn get_render(&self, num: usize) -> Option<String> {
        self.lines.get(num).map(|el| el.render.clone())
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
        for ret in file_reader.lines() {
            lines.push(EditorLine::new(ret?));
        }

        self.lines = lines;
        self.filepath = Some(path.clone());
        self.dirty = false;

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
        self.dirty = false;

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
            lines.push(EditorLine::new(line.to_string()));
        }

        self.lines = lines;
        self.filepath = None;
        self.dirty = false;
    }

    pub fn insert_line(&mut self, cy: usize, line: String) {
        self.lines.insert(cy, EditorLine::new(line.to_string()));
        self.dirty = true;
    }

    pub fn insert_char(&mut self, cx: usize, cy: usize, c: char) {
        if let Some(el) = self.lines.get_mut(cy) {
            el.insert_char(cx, c);
            self.dirty = true;
        }
    }

    pub fn delete_char(&mut self, cx: usize, cy: usize) {
        if let Some(el) = self.lines.get_mut(cy) {
            if cx < el.raw.len() {
                el.remove_char(cx);
                self.dirty = true;
            }
        }
    }

    pub fn delete_line(&mut self, cy: usize) {
        self.lines.remove(cy);
        self.dirty = true;
    }

    pub fn replace_line(&mut self, cy: usize, new_line: String) {
        self.lines[cy] = EditorLine::new(new_line);
    }

    pub fn append_string(&mut self, cx: usize, cy: usize, message: String) {
        if let Some(el) = self.lines.get_mut(cy) {
            el.insert_str(cx, &message);
            self.dirty = true;
        }
    }
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::EditorLine;

    #[test]
    fn test_convert_render() {
        let el = EditorLine::new("".to_string());

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
}
