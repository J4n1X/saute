#![deny(rust_2018_idioms)]
mod ResMan;
use sdl2;
use ResMan::ResourceLoader;
use ResMan::ResourceManager;

use sdl2::event::Event;
use sdl2::event::EventType;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;
use sdl2::video::Window;
use sdl2::video::WindowContext;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

const ANSI_CHAR_RANGE: u32 = 0x80;
const FONT_FILE: &'static str = "Arial.ttf";
const FONT_SIZE: u32 = 64;
const FONT_SPACING: u32 = 2 * (FONT_SIZE / 64); // scales with font_size
const ATLAS_MAX_WIDTH: u32 = 16384;
const ATLAS_MAX_HEIGHT: u32 = 16384;

type RefTexture<'a> = Rc<RefCell<Texture<'a>>>;

#[derive(Default, Debug, Clone)]
pub struct AtlasEntry {
    pub x: u32,
    pub y: u32,
    pub height: u32,
    pub width: u32,
    pub _ax: u32,
    pub _ay: u32,
    pub bl: i32,
    pub bt: i32,
}

impl AtlasEntry {
    pub fn bbox(&self, min_width: u32) -> Rect {
        let corr_width = if self.width > 0 {
            self.width
        } else {
            min_width
        };
        return Rect::new(self.x as i32, self.y as i32, corr_width, self.height);
    }
}

#[derive(Default, Clone)]
pub struct FontDef {
    pub glyph_height: u32,
    pub whitespace_width: u32,
    pub char_spacing: u32,
    pub char_lookup: HashMap<usize, Rc<AtlasEntry>>,
    pub ascender: i32,
    pub descender: i32,
    pub max_ascent: u32,
    pub max_descent: u32,
}

impl FontDef {
    pub fn new(
        char_lookup: HashMap<usize, Rc<AtlasEntry>>,
        max_height: u32,
        char_spacing: u32,
        ascender: i32,
        descender: i32,
        max_ascent: u32,
        max_descent: u32,
    ) -> FontDef {
        let avg_width: u32 =
            char_lookup.values().map(|x| x.width).sum::<u32>() / char_lookup.len() as u32;
        FontDef {
            char_lookup,
            char_spacing,
            ascender,
            descender,
            glyph_height: max_height,
            whitespace_width: avg_width,
            max_ascent,
            max_descent,
        }
    }
    /// Get the corrected position of a character
    /// TODO: cache this information
    pub fn get_char_aligned_rect<A: Into<i32>>(&self, x: A, y: A, info: &AtlasEntry) -> Rect {
        let x: i32 = x.into();
        let y: i32 = y.into();
        let w = if info.width > 1 {
            info.width as u32
        } else {
            self.whitespace_width
        };
        let h = info.height as u32;

        //let align_lowest = self.glyph_height as i32 - info.height as i32;

        let baseline_dist = self.max_ascent as i32 - info.bt;
        Rect::new(x, y + baseline_dist, w, h)
    }

    /// Get the position of the character in the texture atlas
    pub fn get_char<A: Into<usize>>(&self, char: A) -> Result<Rc<AtlasEntry>, ()> {
        let char: usize = char.into();
        if let Some(info) = self.char_lookup.get(&char) {
            Ok(info.clone())
        } else {
            Err(())
        }
    }
}

type TextureManager<'a, T> = ResourceManager<'a, usize, Texture<'a>, TextureCreator<T>>;
impl<'a, T> ResourceLoader<'a, Texture<'a>> for TextureCreator<T> {
    type Args = Surface<'a>;
    fn load(&'a self, arg: &Self::Args) -> Result<Texture<'a>, String> {
        match arg.as_texture(self) {
            Ok(tex) => Ok(tex),
            Err(err) => Err(format!("Failed to load texture from surface: {err}")),
        }
    }
    fn create(&'a self, w: u32, h: u32) -> Texture<'a> {
        self.create_texture_target(PixelFormatEnum::RGB24, w, h)
            .unwrap()
    }
}

pub struct Renderer<'a> {
    canvas: Canvas<Window>,
    texture_manager: TextureManager<'a, WindowContext>,
    loaded_font: FontDef,
    width: u32,
    height: u32,
}

impl<'a> Renderer<'a> {
    pub fn new(
        canvas: Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        width: u32,
        height: u32,
    ) -> Self {
        Renderer {
            canvas: canvas,
            loaded_font: FontDef::default(),
            texture_manager: TextureManager::new(&texture_creator),
            width,
            height,
        }
    }

    pub fn build_atlas<A: Into<String>>(&mut self, font_path: A) {
        use freetype::face::LoadFlag;
        use freetype::Library;
        //let cached_chars: [u8; font_size] = (0..128).collect::<Vec<_>>().try_into().expect("Wrong size iterator");
        let mut max_ascent: u32 = 0;
        let mut max_descent: u32 = 0;

        let lib = Library::init()
            .map_err(|err| {
                eprintln!("Could not initialize FreeType: {err}");
            })
            .unwrap();

        // load first font in ttf file
        let font_face = lib
            .new_face(font_path.into(), 0)
            .map_err(|err| {
                eprintln!("Could not load font: {err}");
            })
            .unwrap();
        //font_face.set_char_size(40*64, 0, 96, 96).unwrap();
        font_face
            .set_pixel_sizes(FONT_SIZE, 0)
            .map_err(|err| {
                eprintln!("Failed to set pixel sizes: {err}");
            })
            .unwrap();
        font_face
            .load_glyph(0, LoadFlag::RENDER)
            .map_err(|err| eprintln!("Could not load first glyph from font: {err}"))
            .unwrap();

        let mut map: HashMap<usize, Rc<AtlasEntry>> = Default::default();
        let metrics = font_face
            .size_metrics()
            .expect("Could not get font metrics: No value returned.");

        let atlas_glyph_height = metrics.height as u32 >> 6;

        let mut _atlas_rows = 0;
        let mut _atlas_cols = 0;
        let mut _atlas_width = 0;
        let glyph_total_width = ANSI_CHAR_RANGE * FONT_SIZE;
        if glyph_total_width > ATLAS_MAX_WIDTH {
            _atlas_rows = (glyph_total_width / ATLAS_MAX_WIDTH) + 1;
            _atlas_cols = ATLAS_MAX_WIDTH / FONT_SIZE;
            _atlas_width = ATLAS_MAX_WIDTH;
        } else if glyph_total_width % ATLAS_MAX_WIDTH == 0 {
            _atlas_rows = glyph_total_width / ATLAS_MAX_WIDTH;
            _atlas_cols = ATLAS_MAX_WIDTH / FONT_SIZE;
            _atlas_width = ATLAS_MAX_WIDTH;
        } else {
            _atlas_rows = 1;
            _atlas_cols = ANSI_CHAR_RANGE;
            _atlas_width = glyph_total_width;
        };
        let atlas_height = atlas_glyph_height * _atlas_rows;

        if atlas_height > ATLAS_MAX_HEIGHT {
            panic!("Texture size exceeded limit of {ATLAS_MAX_WIDTH}x{ATLAS_MAX_HEIGHT}");
        }

        let mut master_surface: Surface<'_> =
            Surface::new(_atlas_width, atlas_height, PixelFormatEnum::RGB24)
                .map_err(|err| {
                    eprintln!("Could not create atlas surface: {err}");
                })
                .unwrap();

        let src = Rect::new(0, 0, FONT_SIZE, atlas_glyph_height);

        for y in 0.._atlas_rows {
            for x in 0.._atlas_cols {
                let ch = y * _atlas_rows + x;
                font_face
                    .load_char(ch as usize, LoadFlag::RENDER)
                    .map_err(|err| eprintln!("Could not load char: {err}"))
                    .unwrap();

                let glyph = font_face.glyph();
                if glyph.bitmap_top() > max_ascent as i32 {
                    max_ascent = glyph.bitmap_top() as u32;
                }
                if ((glyph.metrics().height as i32 >> 6) - glyph.bitmap_top()) > max_descent as i32
                {
                    max_descent =
                        ((glyph.metrics().height as i32 >> 6) - glyph.bitmap_top()) as u32;
                }

                let mut rgb = Vec::<u8>::with_capacity(glyph.bitmap().buffer().len() * 3);
                for pixel in glyph.bitmap().buffer() {
                    rgb.extend_from_slice(&[*pixel, *pixel, *pixel]);
                }

                // loading and blittering this on the CPU should be plenty fast
                let letter = Surface::from_data(
                    &mut rgb[..],
                    glyph.bitmap().width() as u32,
                    glyph.bitmap().rows() as u32,
                    glyph.bitmap().pitch() as u32 * 3,
                    PixelFormatEnum::RGB24,
                )
                .unwrap();
                let dest = Rect::new(
                    (x * FONT_SIZE) as i32,
                    (y * atlas_glyph_height) as i32,
                    FONT_SIZE,
                    atlas_glyph_height,
                );
                letter
                    .blit(src, &mut master_surface, dest)
                    .map_err(|err| {
                        eprintln!("Could not blit to texture atlas: {err}");
                    })
                    .unwrap();

                // add to map
                let entry = AtlasEntry {
                    _ax: glyph.advance().x as u32,
                    _ay: glyph.advance().y as u32,
                    x: x * FONT_SIZE,
                    y: y * atlas_height,
                    height: glyph.metrics().height as u32 >> 6,
                    width: glyph.metrics().width as u32 >> 6,
                    bl: glyph.bitmap_left(),
                    bt: glyph.bitmap_top(),
                };
                map.insert(ch as usize, Rc::new(entry));
            }
        }
        // let master_canvas = master_surface
        //     .into_canvas()
        //     .map_err(|err| eprintln!("Could not create atlas canvas: {err}"))
        //     .unwrap();
        let tex = self
            .texture_manager
            .load(usize::MAX, &master_surface)
            .map_err(|err| {
                eprintln!("Could not create texture from surface: {err}");
            })
            .unwrap();

        dbg!(max_ascent);
        dbg!(max_descent);
        self.loaded_font = FontDef::new(
            map,
            max_ascent + max_descent,
            FONT_SPACING,
            font_face.ascender() as i32 >> 6,
            font_face.descender() as i32 >> 6,
            max_ascent,
            max_descent,
        );
    }

    //pub fn font(&self) -> &FontDef {
    //    if let Some(_) = self._font {
    //        &self._font.as_ref().unwrap()
    //    } else {
    //        panic!("No font loaded!");
    //    }
    //}

    pub fn render_line<A: Into<String>>(
        &mut self,
        y_coord: u32,
        x_offset: u32,
        text: A,
    ) -> Result<RefTexture<'a>, (RefTexture<'a>, String)> {
        let mut text: String = text.into();

        let mut x = x_offset;
        let atlas = self.texture_manager.get(&(usize::MAX)).unwrap();
        let tex = self
            .texture_manager
            .create(y_coord as usize, self.width, self.loaded_font.glyph_height)
            .unwrap();
        let mut result: Option<Result<RefTexture<'a>, (RefTexture<'a>, String)>> = None;
        self.canvas
            .with_texture_canvas(&mut (*tex).borrow_mut(), |canvas| {
                canvas.set_draw_color(Color::RGB(255, 0, 0));
                canvas.fill_rect(Rect::new(
                    x_offset as i32,
                    (y_coord * self.loaded_font.glyph_height) as i32,
                    self.width,
                    self.loaded_font.glyph_height,
                ));
                let mut chars = text.chars();
                while let Some(ch) = chars.next() {
                    let info = self.loaded_font.get_char(ch as usize).unwrap();
                    //println!(
                    //    "Rendering char {ch} with atlas pos {px} and {py}",
                    //    px = info.x,
                    //    py = info.y
                    //);
                    let src = info.bbox(self.loaded_font.whitespace_width);

                    if x + info.width + self.loaded_font.char_spacing > self.width {
                        let rem: String = String::from(ch) + &chars.collect::<String>();
                        result = Some(Err((tex.clone(), rem)));
                        return;
                    }

                    let dest = self.loaded_font.get_char_aligned_rect(
                        x as i32,
                        0 as i32, //y_coord as i32 * self.loaded_font.glyph_height as i32,
                        &info,
                    );
                    dbg!(dest);
                    canvas.copy(&atlas.borrow(), src, dest).unwrap();
                    x += src.width() + self.loaded_font.char_spacing;
                }
                result = Some(Ok(tex.clone()))
            })
            .unwrap();
        return result.unwrap();
    }

    /// Fills the surface with text, starting from (0, 0)
    pub fn render_to_canvas<A: Into<String>>(&mut self, text: A) -> Result<Rect, ()> {
        let text = text.into();
        let mut y_coord: u32 = 0;
        let mut render_text = text;

        'exit: loop {
            match self.render_line(y_coord, 0, &render_text) {
                Ok(tex) => {
                    self.canvas
                        .copy(
                            &tex.borrow(),
                            None,
                            Rect::new(
                                0,
                                (y_coord * self.loaded_font.glyph_height) as i32,
                                self.width,
                                self.loaded_font.glyph_height,
                            ),
                        )
                        .unwrap();
                    break 'exit;
                }
                Err((tex, rem)) => {
                    render_text = rem;
                    self.canvas
                        .copy(
                            &tex.borrow(),
                            None,
                            Rect::new(
                                0,
                                (y_coord * self.loaded_font.glyph_height) as i32,
                                self.width,
                                self.loaded_font.glyph_height,
                            ),
                        )
                        .unwrap();
                }
            }
            y_coord += 1;
        }
        Ok(Rect::new(
            0,
            0,
            self.width,
            y_coord * self.loaded_font.glyph_height,
        ))
    }
}

//pub fn reinit_window_surface(window_surface: &mut WindowSurfaceRef, )

pub fn main() -> Result<(), ()> {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context
        .video()
        .map_err(|err| eprintln!("Could not create video context: {err}"))
        .unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", WIDTH, HEIGHT)
        .position_centered()
        .build()
        .map_err(|err| eprintln!("Could not build window: {err}"))
        .unwrap();

    let window_canvas = window
        .into_canvas()
        .build()
        .map_err(|err| {
            eprintln!("Failed to get window canvas: {err}");
        })
        .unwrap();
    let texman = window_canvas.texture_creator();

    let mut renderer = Renderer::new(window_canvas, &texman, WIDTH, HEIGHT);
    renderer.build_atlas(FONT_FILE);
    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|err| eprintln!("Failed to get event pump: {err}"))
        .unwrap();
    event_pump.enable_event(EventType::TextInput);
    // let mut window_surface = window
    //     .surface()
    //     .map_err(|err| {
    //         eprintln!("Failed to get window surface: {err}");
    //     })
    //     .unwrap();
    let atlas = renderer.texture_manager.get(&usize::MAX).unwrap();
    renderer.canvas.copy(&(*atlas).borrow_mut(), None, None);
    renderer.canvas.present();

    let mut text_buffer: String = "".into();
    let mut need_update: bool = true;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown { keycode, .. } => {
                    println!("[INFO] Event::KeyDown triggered");
                    if let Some(code) = keycode {
                        if code == Keycode::Backspace {
                            text_buffer.pop();
                            need_update = true;
                        }
                    }
                }
                Event::TextInput { text, .. } => {
                    println!("[INFO] Event::TextInput triggered");
                    text_buffer.push_str(&text);
                    need_update = true;
                }
                Event::Window { win_event, .. } => {
                    use sdl2::event::WindowEvent;
                    match win_event {
                        WindowEvent::SizeChanged(w, h) | WindowEvent::Resized(w, h) => {
                            println!(
                                "[INFO] Window resized to {w}x{h}, need to reinit window surface"
                            );
                            //window_surface
                            //    .finish()
                            //    .map_err(|err| eprintln!("Could not close window surface: {err}"))
                            //    .unwrap();
                            //window_surface = window.surface().unwrap();
                            need_update = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if need_update {
            // HACK: Trick to get backspace to work somewhat.
            // Need to fix this when proper line management is running.
            renderer.canvas.set_draw_color::<_>(Color::RGB(0, 0, 0));
            renderer.canvas.clear();
            renderer.render_to_canvas(&text_buffer)?;
            renderer.canvas.present();
            need_update = false;
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}
