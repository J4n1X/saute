use std::rc::Rc;

use sdl2::rect::Rect;

use crate::{FontChar, Renderer};
pub trait Renderable {
    fn render(&self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String>;
}

#[derive(Default, Clone)]
pub struct ScreenLine {
    content: Vec<Rc<FontChar>>,
    width: u32,
    row: usize,
}

impl ScreenLine {
    pub fn new(row: usize) -> Self {
        ScreenLine {
            row,
            ..Default::default()
        }
    }
    pub fn get_text(&self) -> String {
        let str = self.content.iter().map(|fch| fch.ch).collect::<String>();
        str
    }
    #[inline]
    pub fn content(&self) -> &Vec<Rc<FontChar>> {
        &self.content
    }
    pub fn push_char(&mut self, fch: Rc<FontChar>) {
        self.width += fch.bbox.width();
        self.content.push(fch);
    }
    pub fn pop_char(&mut self) -> Option<Rc<FontChar>> {
        self.content.pop()
    }
    pub fn wrapped_bbox(&self, max_width: u32, row_height: u32) -> Rect {
        let height = (self.width as f32 / max_width as f32).ceil() as u32 * row_height;
        Rect::new(0, 0, self.width.clamp(0, max_width), height)
    }
}

impl Renderable for ScreenLine {
    // one line can wrap multiple screen lines!
    fn render(&self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String> {
        let mut w = 0;
        let mut x_offset = 0;
        let mut y_offset = y + self.row as u32 * target.loaded_font.glyph_height;
        for fch in &self.content {
            let ch_w = if fch.bbox.width() <= 1 {
                target.loaded_font.whitespace_width
            } else {
                fch.bbox.width()
            };

            if x_offset + ch_w > target.width {
                y_offset += target.loaded_font.glyph_height;
                x_offset = 0;
            }

            // TODO: Make this generalized!
            if !fch.ch.is_whitespace() {
                fch.render(target, x + x_offset, y_offset)
                    .map_err(|err| {
                        eprintln!("Could not render character: {err}");
                    })
                    .unwrap();
            }
            x_offset += ch_w;
            if x_offset < target.width {
                w = x_offset;
            } else {
                w = target.width;
            }
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
    //lines: Vec<ScreenLine>,
    content: Vec<Rc<FontChar>>,
    width: usize,
    height: usize,
    row_height: usize,
    cursor_abs: u32,
    cursor_col: u32,
    cursor_row: u32,
    highlight_mark: u32,
    _cursor_enabled: bool,
}

impl TextScreen {
    fn put_cursor(&self, target: &mut Renderer<'_>, x: i32, y: i32) {
        if self._cursor_enabled {
            target
                .canvas
                .set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
            let dst = Rect::new(
                x,
                y,
                target.loaded_font.glyph_width / 16,
                target.loaded_font.glyph_height,
            );
            target.canvas.fill_rect(dst).unwrap();
        }
    }

    pub fn new(width: usize, height: usize, row_height: usize) -> Self {
        TextScreen {
            width,
            height,
            row_height,
            highlight_mark: u32::MAX,
            ..Default::default()
        }
    }
    // #region Getters And Setters
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
    #[inline]
    pub fn set_width(&mut self, new_width: usize) {
        self.width = new_width;
    }
    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }
    #[inline]
    pub fn set_height(&mut self, new_height: usize) {
        self.height = new_height;
    }
    #[inline]
    pub fn cursor_enable(&mut self) {
        self._cursor_enabled = true;
    }
    #[inline]
    pub fn cursor_disable(&mut self) {
        self._cursor_enabled = false;
    }
    #[inline]
    pub fn cursor_enabled(&self) -> bool {
        self._cursor_enabled
    }

    #[inline]
    pub fn set_cursor_row(&mut self, row: u32) {
        self.cursor_row = row;
    }
    #[inline]
    pub fn set_cursor_col(&mut self, col: u32) {
        self.cursor_col = col;
    }

    #[inline]
    pub fn set_highlight_mark(&mut self, pos: u32) {
        self.highlight_mark = pos;
    }
    #[inline]
    pub fn set_cursor_abs(&mut self, pos: u32) {
        self.cursor_abs = pos;
    }

    #[inline]
    pub fn get_cursor_row(&mut self) -> u32 {
        self.cursor_row
    }

    #[inline]
    pub fn get_cursor_col(&mut self) -> u32 {
        self.cursor_col
    }

    #[inline]
    pub fn get_highlight_mark(&mut self) -> u32 {
        self.highlight_mark
    }

    #[inline]
    pub fn get_cursor_abs(&mut self) -> u32 {
        self.cursor_abs
    }

    pub fn get_text(&self) -> String {
        self.content.iter().map(|fch| fch.ch).collect::<String>()
    }
    // #endregion
    pub fn cursor_forward(&mut self) {
        if let Some(fch) = self.content.get((self.cursor_abs) as usize) {
            if fch.ch == '\n' {
                println!("New line!");
                self.cursor_col = 0;
                self.cursor_row += 1;
            } else {
                self.cursor_col += 1;
            }
            self.cursor_abs += 1;
        }
    }

    pub fn cursor_back(&mut self) {
        if let Some(_) = self.content.get((self.cursor_abs - 1) as usize) {
            if self.cursor_col == 0 {
                self.cursor_row -= 1;
                self.cursor_col = self
                    .content
                    .iter()
                    .rev()
                    .take_while(|x| x.ch != '\n')
                    .count() as u32;
            }
            self.cursor_col -= 1;
            self.cursor_abs -= 1;
        }
    }

    #[inline]
    pub fn push_char(&mut self, fch: Rc<FontChar>) {
        self.content.insert(self.cursor_abs as usize, fch.clone());
        self.cursor_forward();
    }
    #[inline]
    pub fn pop_char(&mut self) -> Option<Rc<FontChar>> {
        self.cursor_back();
        if self.content.len() != 0 {
            return Some(self.content.remove(self.cursor_abs as usize));
        }
        None
    }
    #[inline]
    pub fn push_string<T: Into<Vec<Rc<FontChar>>>>(&mut self, fstr: T) {
        let fstr: Vec<Rc<FontChar>> = fstr.into();
        for fch in fstr {
            self.push_char(fch);
        }
    }
    #[inline]
    pub fn clear(&mut self) {
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.cursor_abs = 0;
        self.content.clear();
    }

    pub fn render_highlight(target: &mut Renderer<'_>, region: Rect) {
        use sdl2::pixels::Color;
        use sdl2::render::BlendMode;
        let highlight_color = Color::RGB(50, 50, 50);

        target.canvas.set_blend_mode(BlendMode::Add);
        target.canvas.set_draw_color(highlight_color);
        target
            .canvas
            .fill_rect(region)
            .map_err(|err| {
                eprintln!(
                    "Could not highlight at {xpos} x {ypos}: {err}",
                    xpos = region.x(),
                    ypos = region.y()
                );
            })
            .unwrap();
        target.canvas.set_blend_mode(BlendMode::None);
    }

    pub fn render_all(
        &mut self,
        target: &mut Renderer<'_>,
        x: u32,
        y: u32,
    ) -> Result<Rect, String> {
        let mut cur_abs = 0u32;
        let mut y_offset = 0u32;
        let mut x_offset = 0u32;
        for fch in &self.content {
            // decide if we must render or not, we do not want whitespaces to be rendered.
            let dst = if fch.ch.is_whitespace() {
                target.loaded_font.get_char_aligned_rect(
                    (x + x_offset) as i32,
                    (y + y_offset) as i32,
                    &fch,
                )
            } else {
                fch.render(target, x + x_offset, y + y_offset)
                    .map_err(|err| {
                        eprintln!("Failed to render character {ch}: {err}", ch = fch.ch);
                    })
                    .unwrap()
            };

            cur_abs += 1;
            x_offset += dst.width();

            // Extend the highlight region on this line
            if self.cursor_enabled() && self.highlight_mark < cur_abs && cur_abs <= self.cursor_abs
            {
                Self::render_highlight(target, dst);
            }

            // Line wrap and newline logic
            if x + x_offset + fch._ax > self.width as u32 || fch.ch == '\n' {
                x_offset = 0;
                y_offset += self.row_height as u32;
            }

            // Render the cursor if we are at the right place
            if self.cursor_enabled() && self.cursor_abs == cur_abs {
                dbg!(x_offset, y_offset);
                self.put_cursor(target, (x + x_offset) as i32, (y + y_offset) as i32);
            }
        }
        Ok(Rect::new(x as i32, y as i32, x + x_offset, y_offset))
    }
}
