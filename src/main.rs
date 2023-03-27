extern crate sdl2; 

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;
use sdl2::surface::Surface;
use sdl2::rect::Rect;
use std::path::Path;
use std::time::Duration;


fn render_atlas<'a>(w: u32, h: u32) -> Result<Surface<'a>, ()> {
    const font_size: u32 = 128;
    use freetype::Library;
    use freetype::face::LoadFlag;
    use sdl2::pixels::PixelFormatEnum;
    //let cached_chars: [u8; font_size] = (0..128).collect::<Vec<_>>().try_into().expect("Wrong size iterator");

    let lib = Library::init().map_err(|err| {
        eprintln!("Could not initialize FreeType: {err}");
    })?;

    // load first font in ttf file
    let font_face = lib.new_face("Arial.ttf", 0).map_err(|err|{
        eprintln!("Could not load font: {err}");
    })?;
    //font_face.set_char_size(40*64, 0, 96, 96).unwrap();
    font_face.set_pixel_sizes(font_size, 0);

    let mut atlas_height: u32 = font_face.height() as u32;
    let mut atlas_width: u32 = font_face.num_glyphs() as u32 * font_size ;
    let mut master_surface: Surface<'a> = 
        Surface::new(atlas_width, atlas_height, PixelFormatEnum::RGB24).map_err(|err| {
            eprintln!("Could not create atlas surface: {err}");
        })?;
    
    let mut offset = 0; 
    let src  = Rect::new(0, 0, font_size, atlas_height);
    for i in 0..font_face.num_glyphs() {
        font_face.load_glyph(i as u32, LoadFlag::RENDER);
        let glyph = font_face.glyph();
        let mut rgb = Vec::<u8>::with_capacity(glyph.bitmap().buffer().len() * 3);
        for pixel in glyph.bitmap().buffer(){
            rgb.extend_from_slice(&[*pixel, *pixel, *pixel]);
            //rgb.push(*pixel);
        }

        let letter = Surface::from_data(
            &mut rgb[..], 
            glyph.bitmap().width() as u32, 
            glyph.bitmap().rows() as u32, 
            glyph.bitmap().pitch() as u32 * 3,
            PixelFormatEnum::RGB24
        ).unwrap();

        let dest = Rect::new(offset, 0, font_size, atlas_height);
        letter.blit(src, &mut master_surface, dest).map_err(|err| {
            eprintln!("Could not blit to texture atlas: {err}");
        });
    }
    return Ok(master_surface);
    //println!("{:?} {rgb:?}", glyph.bitmap().pixel_mode());


    println!("This font has {count} glyphs", count = font_face.num_glyphs());
    todo!();
}

pub fn main() -> Result<(), ()> {
    const width: u32 = 600;
    const height: u32 = 800;
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo", height, width)
        .position_centered()
        .build()
        .unwrap();
 
    use freetype::Library;
    use freetype::face::LoadFlag;
    use sdl2::pixels::PixelFormatEnum;
    //let cached_chars: [u8; font_size] = (0..128).collect::<Vec<_>>().try_into().expect("Wrong size iterator");

    let lib = Library::init().map_err(|err| {
        eprintln!("Could not initialize FreeType: {err}");
    })?;

    // load first font in ttf file
    let font_face = lib.new_face("Arial.ttf", 0).map_err(|err|{
        eprintln!("Could not load font: {err}");
    })?;
    //font_face.set_char_size(40*64, 0, 96, 96).unwrap();
    font_face.set_pixel_sizes(128, 0);
    font_face.load_char('A' as usize, LoadFlag::RENDER).unwrap();
    let glyph = font_face.glyph();
    let mut rgb = Vec::<u8>::with_capacity(glyph.bitmap().buffer().len() * 3);
    for pixel in glyph.bitmap().buffer(){
        rgb.extend_from_slice(&[*pixel, *pixel, *pixel]);
        //rgb.push(*pixel);
    }

    println!("{:?} {rgb:?}", glyph.bitmap().pixel_mode());
    let letter = Surface::from_data(
        &mut rgb[..], 
        glyph.bitmap().width() as u32, 
        glyph.bitmap().rows() as u32, 
        glyph.bitmap().pitch() as u32 * 3,
        PixelFormatEnum::RGB24
    ).unwrap();
    let rect = Rect::new(0, 0, letter.width(), letter.height());
    let mut canvas = window.into_canvas().build().unwrap();
    let tex_man = canvas.texture_creator();
    let tex = tex_man.create_texture_from_surface(&letter).unwrap();

    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();
    canvas.copy(&tex, rect, Rect::new(0, 0, rect.width() * 2, rect.height() * 2));

    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        //i = (i + 1) % 255;
        //canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        //canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}
