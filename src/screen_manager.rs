use sdl2::rect::Rect;

use crate::Renderer;
pub trait Renderable {
    fn render(&mut self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String>;
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
    fn render(&mut self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String> {
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
                w = x_offset;
            } else {
                w = target.width;
            }
            dbg!(w);
        }
        Ok(Rect::new(
            x as i32,
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
    cursor_x: i32,
    cursor_y: i32,
    _cursor_enabled: bool,
}

impl TextScreen {
    pub fn new(width: usize) -> Self {
        TextScreen {
            width,
            ..Default::default()
        }
    }
    #[inline(always)]
    pub fn lines(&self) -> &Vec<ScreenLine> {
        &self.lines
    }
    #[inline(always)]
    pub fn width(&self) -> usize {
        self.width
    }
    #[inline(always)]
    pub fn set_width(&mut self, new_width: usize) {
        self.width = new_width;
    }
    #[inline(always)]
    pub fn rows(&self) -> usize {
        self.rows
    }
    #[inline(always)]
    pub fn cur_line(&self) -> Option<&ScreenLine> {
        self.lines.last()
    }
    #[inline(always)]
    pub fn cursor_enable(&mut self) {
        self._cursor_enabled = true;
    }
    #[inline(always)]
    pub fn cursor_disable(&mut self) {
        self._cursor_enabled = false;
    }
    #[inline(always)]
    pub fn cursor_enabled(&self) -> bool {
        self._cursor_enabled
    }
    #[inline(always)]
    pub fn get_cursor(&self) -> (i32, i32) {
        return (self.cursor_x, self.cursor_y);
    }

    pub fn set_cursor(&mut self, x: i32, y: i32) -> Result<(), String> {
        if self.width < x as usize {
            return Err(format!(
                "Cursor position out of bounds: Position is at X = {x}x{y}, but screen is only {w} pixels wide",
                w = self.width
            )
            .to_string());
        }
        self.cursor_x = x;
        self.cursor_y = y;
        Ok(())
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

    fn put_cursor(&mut self, target: &mut Renderer<'_>) {
        if self._cursor_enabled {
            println!("Drawing cursor");
            target
                .canvas
                .set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
            let dst = Rect::new(
                self.cursor_x,
                self.cursor_y,
                target.loaded_font.glyph_width / 16,
                target.loaded_font.glyph_height,
            );
            target.canvas.fill_rect(dst).unwrap();
        }
    }
}

impl Renderable for TextScreen {
    fn render(&mut self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String> {
        let mut y_offset = 0;
        let mut x_offset = 0;
        for line in &mut self.lines {
            let line_rect = line
                .render(target, x, y + y_offset)
                .map_err(|err| {
                    eprintln!("Could not render line: {err}");
                })
                .unwrap();
            if line_rect.height() > target.loaded_font.glyph_height {
                y_offset += line_rect.height() - target.loaded_font.glyph_height;
            }
            x_offset = line_rect.width();
        }
        self.set_cursor((x + x_offset) as i32, (y + y_offset) as i32)
            .unwrap();

        self.put_cursor(target);
        Ok(Rect::new(
            x as i32,
            y as i32,
            self.width.try_into().unwrap(),
            y_offset,
        ))
    }
}
