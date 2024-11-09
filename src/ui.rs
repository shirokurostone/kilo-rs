use std::io::Error;

pub struct Terminal {
    width: usize,
    height: usize,
}

impl Terminal {
    pub fn new() -> Result<Terminal, Error> {
        let mut terminal = Terminal {
            width: 0,
            height: 0,
        };
        terminal.update()?;
        Ok(terminal)
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        let size = crossterm::terminal::size()?;
        let width = size.0 as usize;
        let height = size.1 as usize;

        let updated = width != self.width || height != self.height;

        self.width = width;
        self.height = height;

        Ok(updated)
    }
}

#[derive(Debug, PartialEq)]
pub struct Component {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Component {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Component {
        Component {
            x,
            y,
            width,
            height,
        }
    }
    pub fn set_size(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }

    pub fn x(&self) -> usize {
        self.x
    }

    pub fn y(&self) -> usize {
        self.y
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

impl Default for Component {
    fn default() -> Self {
        Component::new(0, 0, 0, 0)
    }
}

pub trait Drawable {
    fn draw(&self, buf: &mut String) -> Result<(), Error>;
}
