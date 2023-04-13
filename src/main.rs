#![deny(rust_2018_idioms)]
mod res_man;
mod screen_manager;

use res_man::ResourceLoader;
use res_man::ResourceManager;
use sdl2;

use sdl2::event::Event;
use sdl2::event::EventType;
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

use crate::screen_manager::Renderable;

const ANSI_CHAR_RANGE: u32 = 0x80;
const FONT_SIZE: u32 = 32;
const FONT_SPACING: u32 = 2 * (FONT_SIZE / 64); // scales with font_size
const ATLAS_MAX_WIDTH: u32 = 16384;
const ATLAS_MAX_HEIGHT: u32 = 16384;

type RefTexture<'a> = Rc<RefCell<Texture<'a>>>;

#[derive(Debug, Clone)]
pub struct FontChar {
    pub ch: char,
    pub bbox: Rect,
    pub _ax: u32,
    pub _ay: u32,
    pub bl: i32,
    pub bt: i32,
}

impl FontChar {
    fn default() -> Self {
        FontChar {
            ch: 0 as char,
            bbox: Rect::new(0, 0, 0, 0),
            _ax: 0,
            _ay: 0,
            bl: 0,
            bt: 0,
        }
    }
    fn new(ch: char, bbox: Rect, _ax: u32, _ay: u32, bl: i32, bt: i32) -> Self {
        FontChar {
            ch,
            bbox,
            _ax,
            _ay,
            bl,
            bt,
        }
    }
}

impl Renderable for FontChar {
    fn render(&self, target: &mut Renderer<'_>, x: u32, y: u32) -> Result<Rect, String> {
        let dst = target
            .loaded_font
            .get_char_aligned_rect(x as i32, y as i32, self);
        target
            .canvas
            .copy(
                &target
                    .texture_manager
                    .get(&usize::MAX)
                    .unwrap_or_else(|| {
                        panic!("Failed to get texture atlas!");
                    })
                    .clone()
                    .borrow(),
                self.bbox,
                dst,
            )
            .map(|_| dst)
    }
}

#[derive(Default, Clone)]
pub struct FontDef {
    pub glyph_height: u32,
    pub glyph_width: u32,
    pub whitespace_width: u32,
    pub char_spacing: u32,
    pub char_lookup: HashMap<usize, Rc<FontChar>>,
    pub max_ascent: u32,
    pub max_descent: u32,
    pub max_back: u32,
    pub max_forward: u32,
    pub font_pixel_size: u32,
}

impl FontDef {
    pub fn new(
        char_lookup: HashMap<usize, Rc<FontChar>>,
        max_height: u32,
        max_width: u32,
        char_spacing: u32,
        max_ascent: u32,
        max_descent: u32,
        max_back: u32,
        max_forward: u32,
        font_pixel_size: u32,
    ) -> FontDef {
        let avg_width: u32 =
            char_lookup.values().map(|x| x.bbox.width()).sum::<u32>() / char_lookup.len() as u32;
        FontDef {
            char_lookup,
            char_spacing,
            glyph_height: max_height,
            glyph_width: max_width,
            whitespace_width: avg_width,
            max_ascent,
            max_descent,
            max_back,
            max_forward,
            font_pixel_size,
        }
    }
    /// Get the corrected position of a character
    /// TODO: cache this information
    pub fn get_char_aligned_rect(&self, x: i32, y: i32, info: &FontChar) -> Rect {
        let x: i32 = x.into();
        let y: i32 = y.into();

        //let align_lowest = self.glyph_height as i32 - info.height as i32;

        let baseline_dist = self.max_ascent as i32 - info.bt;
        let center_dist: i32 = self.max_forward as i32 + info.bl;

        let w = if info.bbox.width() <= 1 {
            self.whitespace_width
        } else {
            info.bbox.width()
        };
        Rect::new(x + center_dist, y + baseline_dist, w, info.bbox.height())
    }

    /// Get the position of the character in the texture atlas
    pub fn get_char<A: Into<usize>>(&self, char: A) -> Result<Rc<FontChar>, ()> {
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
    _cursor_enabled: bool,
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
            _cursor_enabled: false,
        }
    }

    pub fn render_from_atlas(&mut self, ch: usize, x: i32, y: i32) -> Rect {
        let entry = self
            .loaded_font
            .get_char(ch)
            .map_err(|_| {
                eprintln!("Could not get atlas entry");
            })
            .unwrap();
        let src = entry.bbox;
        let dst = self.loaded_font.get_char_aligned_rect(x, y, &entry);
        let atlas = self.texture_manager.get(&usize::MAX).unwrap();

        self.canvas
            .copy(&atlas.borrow(), src, dst)
            .map_err(|err| {
                eprintln!("Could not copy to canvas: {err}");
            })
            .unwrap();
        dst
    }

    pub fn build_atlas<A: Into<String>>(&mut self, font_path: A, font_size: u32) {
        use freetype::face::LoadFlag;
        use freetype::Library;

        // these variables will be used to determine the effective width and height of a character
        let mut max_ascent: u32 = 0;
        let mut max_descent: u32 = 0;
        let mut max_forward: u32 = 0;
        let mut max_back: u32 = 0;
        let mut max_width: u32 = 0;

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
            .set_pixel_sizes(font_size, 0)
            .map_err(|err| {
                eprintln!("Failed to set pixel sizes: {err}");
            })
            .unwrap();
        font_face
            .load_glyph(0, LoadFlag::RENDER)
            .map_err(|err| eprintln!("Could not load first glyph from font: {err}"))
            .unwrap();

        let mut map: HashMap<usize, Rc<FontChar>> = Default::default();
        let metrics = font_face
            .size_metrics()
            .expect("Could not get font metrics: No value returned.");
        let atlas_glyph_height = metrics.height as u32 >> 6;
        let mut _atlas_rows = 0;
        let mut _atlas_cols = 0;
        let mut _atlas_width = 0;
        let glyph_total_width = ANSI_CHAR_RANGE * font_size;
        if glyph_total_width > ATLAS_MAX_WIDTH {
            _atlas_rows = (glyph_total_width / ATLAS_MAX_WIDTH) + 1;
            _atlas_cols = ATLAS_MAX_WIDTH / font_size;
            _atlas_width = ATLAS_MAX_WIDTH;
        } else if glyph_total_width % ATLAS_MAX_WIDTH == 0 {
            _atlas_rows = glyph_total_width / ATLAS_MAX_WIDTH;
            _atlas_cols = ATLAS_MAX_WIDTH / font_size;
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

        let src = Rect::new(0, 0, font_size, atlas_glyph_height);

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
                if glyph.bitmap_left() > max_back as i32 {
                    max_back = glyph.bitmap_left() as u32;
                }
                if ((glyph.metrics().width as i32 >> 6) - glyph.bitmap_left()) > max_forward as i32
                {
                    max_forward =
                        ((glyph.metrics().width as i32 >> 6) - glyph.bitmap_left()) as u32;
                }
                if (glyph.metrics().width as u32 >> 6) > max_width {
                    max_width = glyph.metrics().width as u32 >> 6;
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
                    (x * font_size) as i32,
                    (y * atlas_glyph_height) as i32,
                    font_size,
                    atlas_glyph_height,
                );
                letter
                    .blit(src, &mut master_surface, dest)
                    .map_err(|err| {
                        eprintln!("Could not blit to texture atlas: {err}");
                    })
                    .unwrap();

                // add to map
                let bbox = Rect::new(
                    (x * FONT_SIZE) as i32,
                    (y * atlas_height) as i32,
                    glyph.metrics().width as u32 >> 6,
                    glyph.metrics().height as u32 >> 6,
                );
                let entry = FontChar::new(
                    char::from_u32(ch as u32).unwrap(),
                    bbox,
                    glyph.advance().x as u32 >> 6,
                    glyph.advance().y as u32 >> 6,
                    glyph.bitmap_left(),
                    glyph.bitmap_top(),
                );
                map.insert(ch as usize, Rc::new(entry));
            }
        }

        self.texture_manager
            .load(usize::MAX, &master_surface)
            .map_err(|err| {
                eprintln!("Could not create texture from surface: {err}");
            })
            .unwrap();

        self.loaded_font = FontDef::new(
            map,
            max_ascent + max_descent,
            max_width, //max_forward + max_back,
            FONT_SPACING,
            max_ascent,
            max_descent,
            font_size,
            max_back,
            max_forward,
        );
    }
}

//pub fn reinit_window_surface(window_surface: &mut WindowSurfaceRef, )

pub fn main() -> Result<(), ()> {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;
    const FONT_FILE: &'static str = "Arial.ttf";

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context
        .video()
        .map_err(|err| eprintln!("Could not create video context: {err}"))
        .unwrap();

    let window = video_subsystem
        .window("Saute Text Editor", WIDTH, HEIGHT)
        .position_centered()
        .resizable()
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
    renderer.build_atlas(FONT_FILE, FONT_SIZE);
    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|err| eprintln!("Failed to get event pump: {err}"))
        .unwrap();

    renderer.canvas.set_draw_color::<_>(Color::RGB(0, 0, 0));
    renderer.canvas.clear();
    renderer.canvas.present();

    event_pump.enable_event(EventType::TextInput);

    let mut screen_man = screen_manager::TextScreen::new(
        WIDTH as usize,
        HEIGHT as usize,
        renderer.loaded_font.glyph_height as usize,
    );
    let mut need_update: bool = true;
    screen_man.cursor_enable();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown { keycode, .. } => {
                    if let Some(code) = keycode {
                        match code {
                            Keycode::Return | Keycode::Return2 => {
                                let fch = renderer
                                    .loaded_font
                                    .get_char('\n' as usize)
                                    .map_err(|_| {
                                        eprintln!("Failed to get char '\\n' from texture atlas");
                                    })
                                    .unwrap();
                                screen_man.push_char(fch);
                                need_update = true;
                            }
                            Keycode::Backspace => {
                                screen_man.pop_char();
                                need_update = true;
                            }
                            Keycode::Right => {
                                screen_man.cursor_forward();
                                need_update = true;
                            }

                            Keycode::Left => {
                                screen_man.cursor_back();
                                need_update = true;
                            }
                            _ => {}
                        }
                    }
                }
                Event::TextInput { text, .. } => {
                    println!("[INFO] Event::TextInput triggered");
                    text.chars().for_each(|ch| {
                        let fch = renderer
                            .loaded_font
                            .get_char(ch as usize)
                            .map_err(|_| {
                                eprintln!("Failed to get char {ch} from texture atlas");
                            })
                            .unwrap();
                        screen_man.push_char(fch);
                    });
                    need_update = true;
                }
                Event::Window { win_event, .. } => {
                    use sdl2::event::WindowEvent;
                    match win_event {
                        WindowEvent::SizeChanged(w, h) | WindowEvent::Resized(w, h) => {
                            println!(
                                "[INFO] Window resized to {w}x{h}, need to reinit window surface"
                            );
                            renderer.width = w as u32;
                            renderer.height = h as u32;
                            screen_man.set_width(w as usize);
                            need_update = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if need_update {
            println!(
                "[INFO] Updating screen! {w} x {h}",
                w = renderer.width,
                h = renderer.height
            );
            renderer.canvas.set_draw_color::<_>(Color::RGB(0, 0, 0));
            renderer
                .canvas
                .fill_rect(Rect::new(0, 0, renderer.width, renderer.height))
                .unwrap();

            screen_man
                .render_all(&mut renderer, 0, 0)
                .map_err(|err| {
                    eprintln!("Could not render to canvas: {err}");
                })
                .unwrap();
            renderer.canvas.present();
            need_update = false;
        }

        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    println!("Final text buffer:\n{text}", text = screen_man.get_text());
    Ok(())
}
