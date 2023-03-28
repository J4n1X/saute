extern crate sdl2;

use sdl2::event::Event;
use sdl2::event::EventType;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::surface::Surface;
use sdl2::surface::SurfaceRef;
use std::collections::HashMap;
use std::time::Duration;

const FONT_FILE: &'static str = "Arial.ttf";
const FONT_SIZE: u32 = 64;
const FONT_SPACING: u32 = 2 * (FONT_SIZE / 64); // scales with font_size

#[derive(Default, Debug)]
pub struct AtlasEntry {
    pub offset: u32,
    pub height: u32,
    pub width: u32,
    pub ax: u32,
    pub ay: u32,
    pub bl: i32,
    pub bt: i32,
}

pub struct FontDef<'a> {
    pub atlas: Surface<'a>,
    pub max_height: u32,
    pub whitespace_width: u32,
    pub char_spacing: u32,
    pub char_lookup: HashMap<usize, AtlasEntry>,
}

impl<'a> FontDef<'a> {
    pub fn new(
        atlas: Surface<'a>,
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
            max_height,
            whitespace_width: avg_width,
        }
    }
}

fn render_atlas<'a, T: Into<String>>(font_path: T) -> Result<FontDef<'a>, ()> {
    use freetype::face::LoadFlag;
    use freetype::Library;
    //let cached_chars: [u8; font_size] = (0..128).collect::<Vec<_>>().try_into().expect("Wrong size iterator");

    let lib = Library::init().map_err(|err| {
        eprintln!("Could not initialize FreeType: {err}");
    })?;

    // load first font in ttf file
    let font_face = lib.new_face(font_path.into(), 0).map_err(|err| {
        eprintln!("Could not load font: {err}");
    })?;
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
    let atlas_height: u32 = metrics.height as u32 / metrics.y_ppem as u32;
    let atlas_width: u32 = 0x80 * FONT_SIZE;
    let mut master_surface: Surface<'a> =
        Surface::new(atlas_width, atlas_height, PixelFormatEnum::RGB24).map_err(|err| {
            eprintln!("Could not create atlas surface: {err}");
        })?;

    let mut offset: u32 = 0;
    let src = Rect::new(0, 0, FONT_SIZE, atlas_height);

    for i in 0..0x80 {
        font_face
            .load_char(i as usize, LoadFlag::RENDER)
            .map_err(|err| eprintln!("Could not load char: {err}"))
            .unwrap();
        let glyph = font_face.glyph();

        let mut rgb = Vec::<u8>::with_capacity(glyph.bitmap().buffer().len() * 3);
        for pixel in glyph.bitmap().buffer() {
            rgb.extend_from_slice(&[*pixel, *pixel, *pixel]);
        }

        let letter = Surface::from_data(
            &mut rgb[..],
            glyph.bitmap().width() as u32,
            glyph.bitmap().rows() as u32,
            glyph.bitmap().pitch() as u32 * 3,
            PixelFormatEnum::RGB24,
        )
        .unwrap();

        let dest = Rect::new(offset as i32, 0, FONT_SIZE, atlas_height);
        letter.blit(src, &mut master_surface, dest).map_err(|err| {
            eprintln!("Could not blit to texture atlas: {err}");
        })?;

        // add to map
        let entry = AtlasEntry {
            ax: glyph.advance().x as u32,
            ay: glyph.advance().y as u32,
            offset: i,
            height: glyph.metrics().height as u32 >> 6,
            width: glyph.metrics().width as u32 >> 6,
            bl: glyph.bitmap_left(),
            bt: glyph.bitmap_top(),
        };
        map.insert(i as usize, entry);
        offset += FONT_SIZE;
    }
    return Ok(FontDef::new(
        master_surface,
        map,
        atlas_height,
        FONT_SPACING,
    ));
    //println!("{:?} {rgb:?}", glyph.bitmap().pixel_mode());
}

/// Render a character to the specified surface at position x and y.
/// Returns a Rect in which the char was rendered.
fn render_char(
    font: &FontDef,
    target_surface: &mut SurfaceRef,
    x: i32,
    y: i32,
    char: u8,
) -> Result<Rect, ()> {
    if let Some(info) = font.char_lookup.get(&(char as usize)) {
        let w = if info.width > 1 {
            info.width as u32
        } else {
            font.whitespace_width
        };
        let h = info.height as u32;

        let vert_align = font.max_height as i32 - info.bt;
        let dest_pos = Rect::new(x, y + vert_align, w, h);
        let src_pos = Rect::new((info.offset * FONT_SIZE) as i32, 0, w, h);
        //let surface_height = font.max_height + vert_align as u32;
        let surface_height = info.height + vert_align as u32;
        if target_surface.height() < surface_height {
            return Err(());
        }
        //let mut surface = Surface::new(w, surface_height, PixelFormatEnum::RGB24).unwrap();
        font.atlas
            .blit(src_pos, target_surface, dest_pos)
            .map_err(|err| eprintln!("Could not blit to surface: {err}"))
            .unwrap();
        Ok(Rect::new(x, y, w, surface_height))
    } else {
        Err(())
    }
}

fn render_line<T>(
    font: &FontDef,
    target_surface: &mut SurfaceRef,
    x: i32,
    y: i32,
    text: T,
) -> Result<Rect, (Rect, String)>
where
    T: Into<String>,
{
    let mut text: String = text.into().chars().rev().collect();
    let mut max_y: u32 = 0;
    let mut x_offset: u32 = 0;
    while let Some(ch) = text.chars().last() {
        let render_len: i32 =
            font.char_lookup.get(&(ch as usize)).unwrap().width as i32 + font.char_spacing as i32;
        if x_offset as i32 + render_len >= target_surface.width() as i32 {
            return Err((
                Rect::new(x, y, x_offset, max_y),
                text.chars().rev().collect(),
            ));
        }
        let render_rect =
            render_char(&font, target_surface, x + x_offset as i32, y, ch as u8).unwrap();
        if render_rect.height() > max_y {
            max_y = render_rect.height();
        }
        x_offset += render_rect.width() + font.char_spacing;
        text.pop();
    }
    Ok(Rect::new(x, y, x_offset, max_y))
}

/// Fills the surface with text, starting from (0, 0)
pub fn render_to_surface<T>(
    font: &FontDef,
    target_surface: &mut SurfaceRef,
    text: T,
) -> Result<Rect, ()>
where
    T: Into<String>,
{
    let text = text.into();

    let mut h: u32 = 0;
    let mut w: u32 = 0;
    let mut render_text = text;
    'render: loop {
        match render_line(font, target_surface, 0, h as i32, &render_text) {
            Ok(rect) => {
                if rect.width() > w {
                    w = rect.width();
                }
                break 'render;
            }
            Err((rect, rem)) => {
                render_text = rem;
                if rect.width() > w {
                    w = rect.width();
                }
                h += rect.height();
            }
        }
    }
    Ok(Rect::new(0, 0, w, h))
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

    let font = render_atlas(FONT_FILE).unwrap();

    let mut text_buffer: String = "".into();
    let mut need_update: bool = false;
    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|err| eprintln!("Failed to get event pump: {err}"))
        .unwrap();
    event_pump.enable_event(EventType::TextInput);
    let mut window_surface = window
        .surface()
        .map_err(|err| {
            eprintln!("Failed to get window surface: {err}");
        })
        .unwrap();

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
                            window_surface
                                .finish()
                                .map_err(|err| eprintln!("Could not close window surface: {err}"))
                                .unwrap();
                            window_surface = window.surface().unwrap();
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
            use sdl2::pixels::Color;
            let cover_rect = window_surface
                .clip_rect()
                .expect("Window surface clip rectangle was <None>");
            window_surface
                .fill_rect(cover_rect, Color::RGB(0, 0, 0))
                .map_err(|err| {
                    eprintln!("Could not clear window surface: {err}");
                })
                .unwrap();
            render_to_surface(&font, &mut window_surface, &text_buffer)?;
            window_surface
                .update_window()
                .map_err(|err| {
                    eprintln!("[ERROR] Failed to update window: {err}");
                })
                .unwrap();
            need_update = false;
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    window_surface.finish().unwrap();
    Ok(())
}
