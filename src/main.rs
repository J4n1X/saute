#![deny(rust_2018_idioms)]

use sdl2;

use sdl2::event::Event;
use sdl2::event::EventType;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::RenderTarget;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;
use sdl2::surface::SurfaceContext;
use sdl2::surface::SurfaceRef;
use sdl2::video::Window;
use sdl2::video::WindowContext;

use std::collections::HashMap;
use std::ops::DerefMut;
use std::time::Duration;

const ANSI_CHAR_RANGE: u32 = 0x80;
const FONT_FILE: &'static str = "Arial.ttf";
const FONT_SIZE: u32 = 64;
const FONT_SPACING: u32 = 2 * (FONT_SIZE / 64); // scales with font_size
const ATLAS_MAX_WIDTH: u32 = 16384;
const ATLAS_MAX_HEIGHT: u32 = 16384;

#[derive(Default, Debug)]
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
    pub fn bbox(&self) -> Rect {
        return Rect::new(self.x as i32, self.y as i32, self.width, self.height);
    }
}

pub struct FontDef<'a> {
    pub atlas: Texture<'a>,
    pub glyph_height: u32,
    pub whitespace_width: u32,
    pub char_spacing: u32,
    pub char_lookup: HashMap<usize, AtlasEntry>,
}

impl<'a> FontDef<'a> {
    pub fn new(
        atlas: Texture<'a>,
        char_lookup: HashMap<usize, AtlasEntry>,
        max_height: u32,
        char_spacing: u32,
    ) -> FontDef<'a> {
        let avg_width: u32 =
            char_lookup.values().map(|x| x.width).sum::<u32>() / char_lookup.len() as u32;
        FontDef {
            atlas,
            char_lookup,
            char_spacing,
            glyph_height: max_height,
            whitespace_width: avg_width,
        }
    }
}

pub struct Renderer {
    canvas: Canvas<Window>,
    texture_manager: TextureCreator<WindowContext>,
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(window: Window, width: u32, height: u32) -> Self {
        let mut window_canvas = window
            .into_canvas()
            .build()
            .map_err(|err| {
                eprintln!("Failed to get window canvas: {err}");
            })
            .unwrap();
        let texture_manager = window_canvas.texture_creator();
        Renderer {
            canvas: window_canvas,
            texture_manager,
            width,
            height,
        }
    }

    pub fn build_atlas<'a, T: Into<String>>(&mut self, font_path: T) -> FontDef<'a> {
        use freetype::face::LoadFlag;
        use freetype::Library;
        //let cached_chars: [u8; font_size] = (0..128).collect::<Vec<_>>().try_into().expect("Wrong size iterator");

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

        let mut map: HashMap<usize, AtlasEntry> = Default::default();
        let metrics = font_face
            .size_metrics()
            .expect("Could not get font metrics: No value returned.");

        let atlas_glyph_height = metrics.height as u32 >> 6;

        let mut atlas_rows = 0;
        let mut atlas_cols = 0;
        let mut atlas_width = 0;
        let glyph_total_width = ANSI_CHAR_RANGE * FONT_SIZE;
        if glyph_total_width > ATLAS_MAX_WIDTH {
            atlas_rows = (glyph_total_width / ATLAS_MAX_WIDTH) + 1;
            atlas_cols = ATLAS_MAX_WIDTH / FONT_SIZE;
            atlas_width = ATLAS_MAX_WIDTH;
        } else if glyph_total_width % ATLAS_MAX_WIDTH == 0 {
            atlas_rows = glyph_total_width / ATLAS_MAX_WIDTH;
            atlas_cols = ATLAS_MAX_WIDTH / FONT_SIZE;
            atlas_width = ATLAS_MAX_WIDTH;
        } else {
            atlas_rows = 1;
            atlas_cols = ANSI_CHAR_RANGE;
            atlas_width = glyph_total_width;
        };
        let atlas_height = atlas_glyph_height * atlas_rows;

        if atlas_height > ATLAS_MAX_HEIGHT {
            panic!("Texture size exceeded limit of {ATLAS_MAX_WIDTH}x{ATLAS_MAX_HEIGHT}");
        }

        let mut master_surface: Surface<'_> =
            Surface::new(atlas_width, atlas_height, PixelFormatEnum::RGB24)
                .map_err(|err| {
                    eprintln!("Could not create atlas surface: {err}");
                })
                .unwrap();

        let src = Rect::new(0, 0, FONT_SIZE, atlas_glyph_height);

        for y in 0..atlas_rows {
            for x in 0..atlas_cols {
                let ch = y * atlas_rows + x;
                font_face
                    .load_char(ch as usize, LoadFlag::RENDER)
                    .map_err(|err| eprintln!("Could not load char: {err}"))
                    .unwrap();
                let glyph = font_face.glyph();

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
                    x,
                    y,
                    height: glyph.metrics().height as u32 >> 6,
                    width: glyph.metrics().width as u32 >> 6,
                    bl: glyph.bitmap_left(),
                    bt: glyph.bitmap_top(),
                };
                map.insert(ch as usize, entry);
            }
        }
        // let master_canvas = master_surface
        //     .into_canvas()
        //     .map_err(|err| eprintln!("Could not create atlas canvas: {err}"))
        //     .unwrap();
        let tex: Texture<'a> = self
            .canvas
            .texture_creator()
            .create_texture_from_surface(master_surface)
            .map_err(|err| {
                eprintln!("Could not render surface to texture: {err}");
            })
            .unwrap();

        return FontDef::new(tex, map, atlas_glyph_height, FONT_SPACING);
        //println!("{:?} {rgb:?}", glyph.bitmap().pixel_mode());
    }

    //pub fn font(&self) -> &FontDef<'_> {
    //    if let Some(_) = self._font {
    //        &self._font.as_ref().unwrap()
    //    } else {
    //        panic!("No font loaded!");
    //    }
    //}

    /// Get the corrected position of a character
    pub fn get_char_aligned_rect<T: Into<i32>>(
        &self,
        font: &FontDef<'_>,
        x: T,
        y: T,
        info: &AtlasEntry,
    ) -> Rect {
        let x: i32 = x.into();
        let y: i32 = y.into();
        let w = if info.width > 1 {
            info.width as u32
        } else {
            font.whitespace_width
        };
        let h = info.height as u32;

        let vert_align = font.glyph_height as i32 - info.bt;
        Rect::new(x, y + vert_align, w, h)
    }

    /// Get the position of the character in the texture atlas
    pub fn get_char<'a, T: Into<usize>>(
        &self,
        font: &'a FontDef<'a>,
        char: T,
    ) -> Result<&'a AtlasEntry, ()> {
        if let Some(info) = font.char_lookup.get(&(char.into())) {
            Ok(info)
        } else {
            Err(())
        }
    }

    pub fn render_line<T: Into<String>>(
        &mut self,
        font: &FontDef<'_>,
        x_offset: u32,
        text: T,
    ) -> Result<&Texture<'_>, (&Texture<'_>, String)> {
        let tex_man = self.canvas.texture_creator();
        let mut tgt_tex: Texture<'_> = self
            .texture_manager
            .create_texture(
                PixelFormatEnum::RGB24,
                sdl2::render::TextureAccess::Target,
                self.width - x_offset,
                font.glyph_height,
            )
            .map_err(|err| {
                eprintln!("Could not create line texture: {err}");
            })
            .unwrap();

        let mut render_res: Result<&Texture<'_>, (&Texture<'_>, String)>;
        self.canvas.with_texture_canvas(&mut tgt_tex, |tex_canvas| {
            let mut text: String = text.into().chars().rev().collect();

            let mut x = x_offset;
            while let Some(ch) = text.chars().last() {
                let render_len: i32 = font.char_lookup.get(&(ch as usize)).unwrap().width as i32
                    + font.char_spacing as i32;
                if x as i32 + render_len >= self.width as i32 {
                    render_res = Err((&tgt_tex, text.chars().rev().collect()));
                }
                let char = self
                    .get_char(font, ch as usize)
                    .map_err(|err| {
                        eprintln!("Could not get {ch} from atlas");
                    })
                    .unwrap();
                let dest_rect = self.get_char_aligned_rect(font, x as i32, 0, char);
                tex_canvas.copy(&font.atlas, char.bbox(), dest_rect);
                //let render_rect = self
                //    .get_char_aligned_rect(x + x_offset as i32, 0, ch as u8)
                //    .unwrap();
                x += dest_rect.width() + font.char_spacing;
                text.pop();
            }
            render_res = Ok(&tgt_tex);
            tex_canvas.present();
        });
        return render_res;
    }

    /// Fills the surface with text, starting from (0, 0)
    pub fn render_to_canvas<T: Into<String>>(
        &mut self,
        font: &FontDef<'_>,
        text: T,
    ) -> Result<Rect, ()> {
        let text = text.into();
        let canvas_rect = self.canvas.clip_rect().unwrap();
        let mut y: u32 = 0;
        let mut render_text = text;

        'exit: loop {
            match self.render_line(font, 0, &render_text) {
                Ok(rect) => {
                    break 'exit;
                }
                Err((tex, rem)) => {
                    render_text = rem;
                    self.canvas.copy(
                        &tex,
                        None,
                        Rect::new(0, y as i32, canvas_rect.width(), font.glyph_height),
                    );
                }
            }
            y += font.glyph_height;
        }
        Ok(Rect::new(0, 0, canvas_rect.width(), y))
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

    let mut renderer = Renderer::new(window, WIDTH, HEIGHT);
    let font = renderer.build_atlas(FONT_FILE);

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
            renderer.canvas.set_draw_color(Color::RGB(0, 0, 0));
            renderer.canvas.clear();
            renderer.render_to_canvas(&font, &text_buffer)?;
            renderer.canvas.present();
            need_update = false;
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}
