extern crate sdl2; 

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;
use sdl2::rect::Rect;
use std::path::Path;
use std::time::Duration;

pub fn main() -> Result<(), ()> {
    let sdl_context = sdl2::init().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // load the font
    let font_path: &Path = Path::new("/home/janick/saute/Arial.ttf"); 
    let font = ttf_context.load_font(&font_path, 24).unwrap();
    let text_surface = font
        .render("Hello, World!")
        .shaded(Color::RGB(255,255,255), Color::RGB(0,0,0))
        .unwrap();



    let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();
 
    let mut canvas = window.into_canvas().build().unwrap();
    let tex_creator = canvas.texture_creator();

    let tex = Texture::from_surface(&text_surface, &tex_creator).unwrap();
    let tex_props = tex.query();
    let dest_rect = Rect::new(0, 0, tex_props.width, tex_props.height); 

    canvas.set_draw_color(Color::RGB(0,0,0));
    canvas.clear();
    canvas.copy(&tex, None, dest_rect); 

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
