extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::surface::Surface;
use std::collections::HashMap;
use std::time::Duration;

const font_size: u32 = 64;

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

pub struct AtlasLookup<'a> {
    pub atlas: Surface<'a>,
    pub max_height: u32,
    pub lowest_base: i32,
    pub char_lookup: HashMap<usize, AtlasEntry>,
}

impl<'a> AtlasLookup<'a> {
    pub fn new(
        atlas: Surface<'a>,
        max_height: u32,
        lowest_base: i32,
        char_lookup: HashMap<usize, AtlasEntry>,
    ) -> AtlasLookup<'a> {
        AtlasLookup {
            atlas: atlas,
            max_height: max_height,
            lowest_base: lowest_base,
            char_lookup,
        }
    }
}

fn render_atlas<'a>() -> Result<AtlasLookup<'a>, ()> {
    use freetype::face::LoadFlag;
    use freetype::Library;
    //let cached_chars: [u8; font_size] = (0..128).collect::<Vec<_>>().try_into().expect("Wrong size iterator");

    let lib = Library::init().map_err(|err| {
        eprintln!("Could not initialize FreeType: {err}");
    })?;

    // load first font in ttf file
    let font_face = lib.new_face("Arial.ttf", 0).map_err(|err| {
        eprintln!("Could not load font: {err}");
    })?;
    //font_face.set_char_size(40*64, 0, 96, 96).unwrap();
    font_face.set_pixel_sizes(font_size, 0);
    font_face.load_glyph(0, LoadFlag::RENDER);

    let mut map: HashMap<usize, AtlasEntry> = Default::default();
    let metrics = font_face.size_metrics().unwrap();
    let atlas_height: u32 = metrics.height as u32 / metrics.y_ppem as u32;
    let atlas_width: u32 = 0x80 * font_size;
    let mut master_surface: Surface<'a> =
        Surface::new(atlas_width, atlas_height, PixelFormatEnum::RGB24).map_err(|err| {
            eprintln!("Could not create atlas surface: {err}");
        })?;

    let mut offset: u32 = 0;
    let src = Rect::new(0, 0, font_size, atlas_height);

    let mut lowest_base: i32 = atlas_height as i32;

    for i in 0..0x80 {
        font_face.load_char(i as usize, LoadFlag::RENDER).unwrap();
        let glyph = font_face.glyph();

        if (glyph.bitmap().rows() as i32) - (glyph.bitmap_top() as i32) < lowest_base {
            lowest_base = (glyph.bitmap().rows() as i32) - (glyph.bitmap_top() as i32);
        }

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

        //let pos: i32 = (atlas_height as i32) - (glyph.metrics().height as i32 >> 6); //glyph.bitmap_top();
        //if i == 11 {
        //    println!(
        //        "{atlas_height} {pos} {top} {h}",
        //        top = glyph.bitmap_top(),
        //        h = glyph.metrics().height >> 6
        //    );
        //}
        let dest = Rect::new(offset as i32, 0, font_size, atlas_height);
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
        offset += font_size;
    }
    return Ok(AtlasLookup::new(
        master_surface,
        atlas_height,
        lowest_base,
        map,
    ));
    //println!("{:?} {rgb:?}", glyph.bitmap().pixel_mode());
}

fn render_char<'a>(font: &AtlasLookup, char: u8) -> Result<Surface<'a>, ()> {
    if let Some(info) = font.char_lookup.get(&(char as usize)) {
        let x = info.bl;
        let y = info.bt;
        let w = info.width as u32;
        let h = info.height as u32;

        let dest_pos = Rect::new(0, (font.max_height as i32 - info.bt), w, h);
        let src_pos = Rect::new((info.offset * font_size) as i32, 0, w, h);

        let surface_height = font.max_height + (font.max_height as u32 - info.bt as u32);
        let mut surface = Surface::new(w, surface_height, PixelFormatEnum::RGB24).unwrap();
        font.atlas.blit(src_pos, &mut surface, dest_pos).unwrap();
        Ok(surface)
    } else {
        Err(())
    }
}

pub fn main() -> Result<(), ()> {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", WIDTH, HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let font = render_atlas().unwrap();
    font.atlas.save_bmp("test.bmp").unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let tex_man = canvas.texture_creator();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    println!("Canvas presented");
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut i = 0;
    'running: loop {
        //i = (i + 1) % 255;
        //canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        //canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown { keycode, .. } => {
                    if let Some(code) = keycode {
                        let ch = code.to_string().to_lowercase().chars().nth(0).unwrap();

                        let mut screen_surface =
                            Surface::new(WIDTH, HEIGHT, PixelFormatEnum::RGB24).unwrap();
                        let target = render_char(&font, ch as u8).unwrap();

                        let src_rect = target.clip_rect().unwrap();
                        let dest_rect = Rect::new(0, 0, src_rect.width(), src_rect.height());

                        target
                            .blit(src_rect, &mut screen_surface, dest_rect)
                            .unwrap();
                        let tex = screen_surface.as_texture(&tex_man).unwrap();
                        canvas.copy(&tex, None, screen_surface.clip_rect()).unwrap();
                        canvas.present();
                    }
                }
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}
