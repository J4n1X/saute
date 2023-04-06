use std::borrow::Borrow;

use sdl2::{
    rect::Rect,
    render::{Canvas, RenderTarget},
    sys::Screen,
};

use crate::{AtlasEntry, Renderer};
pub trait Renderable {
    fn render(&self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String>;
}

#[derive(Default, Clone)]
pub struct ScreenLine {
    text: String,
    row: usize,
}

impl ScreenLine {
    pub fn new(row: usize) -> Self {
        ScreenLine {
            row,
            ..Default::default()
        }
    }
    pub fn text(&self) -> &String {
        &self.text
    }
    pub fn row(&self) -> usize {
        self.row
    }
    pub fn push_char(&mut self, ch: char) {
        self.text.push(ch as char);
    }
    pub fn pop_char(&mut self) -> Option<char> {
        self.text.pop()
    }
}

impl Renderable for ScreenLine {
    // one line can wrap multiple screen lines!
    fn render(&self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String> {
        let mut w = 0;
        let mut x_offset = 0;
        let mut y_offset = y + self.row as u32 * target.loaded_font.glyph_height;
        for ch in self.text.chars() {
            let info = target
                .loaded_font
                .get_char(ch as usize)
                .map_err(|_| {
                    eprintln!("Could not get Atlas entry {ch}");
                })
                .unwrap();

            let info_w = if info.width <= 1 {
                target.loaded_font.whitespace_width
            } else {
                info.width
            };

            if x_offset + info_w > target.width {
                y_offset += target.loaded_font.glyph_height;
                x_offset = 0;
            }

            target.render_from_atlas(ch as usize, (x + x_offset) as i32, y_offset as i32);

            x_offset += info_w;
            if x_offset < target.width {
                w += x_offset;
            } else {
                w = target.width;
            }
        }
        Ok(Rect::new(
            0,
            y_offset as i32,
            w,
            target.loaded_font.glyph_height + y_offset - y,
        ))
    }
}

#[derive(Default, Clone)]
pub struct TextScreen {
    lines: Vec<ScreenLine>,
    width: usize,
    rows: usize,
}

impl TextScreen {
    pub fn new(width: usize) -> Self {
        TextScreen {
            width,
            ..Default::default()
        }
    }
    pub fn lines(&self) -> &Vec<ScreenLine> {
        &self.lines
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn set_width(&mut self, new_width: usize) {
        self.width = new_width;
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn cur_line(&self) -> Option<&ScreenLine> {
        self.lines.last()
    }
    pub fn push_char(&mut self, ch: char) {
        if let Some(cur_line) = self.lines.last_mut() {
            if (ch == '\n') {
                self.lines.push(ScreenLine::new(self.rows));
                self.rows += 1;
            } else {
                cur_line.push_char(ch);
            }
        } else {
            let new_line = ScreenLine::new(self.rows);
            self.lines.push(new_line);
            self.rows += 1;

            // recursive push
            self.push_char(ch);
        }
    }
    pub fn pop_char(&mut self) -> Option<char> {
        if let Some(cur_line) = self.lines.last_mut() {
            if cur_line.text().is_empty() {
                self.lines.pop();
                self.rows -= 1;
                return Some('\n');
            } else {
                return cur_line.pop_char();
            }
        } else {
            None
        }
    }
}

impl Renderable for TextScreen {
    fn render(&self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String> {
        let mut y_offset = 0;
        for line in &self.lines {
            let line_rect = line
                .render(target, x, y + y_offset)
                .map_err(|err| {
                    eprintln!("Could not render line: {err}");
                })
                .unwrap();
            if line_rect.height() > target.loaded_font.glyph_height {
                y_offset += line_rect.height() - target.loaded_font.glyph_height;
            }
        }
        Ok(Rect::new(
            x as i32,
            y as i32,
            self.width.try_into().unwrap(),
            y_offset,
        ))
    }
}
