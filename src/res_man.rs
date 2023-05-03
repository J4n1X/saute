use std::{cell::RefCell, collections::HashMap, fmt::Display, hash::Hash, rc::Rc};

use sdl2::rect::Rect;

pub trait ResourceLoader<'l, R> {
    type Args: ?Sized;
    fn load(&'l self, data: &Self::Args) -> Result<R, String>;
    fn create(&'l self, w: u32, h: u32) -> R;
}

pub struct ResourceManager<'l, K, R, L>
where
    K: Hash + Eq,
    L: ResourceLoader<'l, R>,
{
    loader: &'l L,
    table: HashMap<K, Rc<RefCell<R>>>,
}

impl<'l, K, R, L> ResourceManager<'l, K, R, L>
where
    K: Hash + Eq,
    L: ResourceLoader<'l, R>,
{
    pub fn new(loader: &'l L) -> Self {
        ResourceManager {
            table: HashMap::new(),
            loader: loader,
        }
    }

    pub fn create<D>(&mut self, key: K, w: u32, h: u32) -> Result<Rc<RefCell<R>>, String>
    where
        L: ResourceLoader<'l, R, Args = D>,
        K: 'l + Hash + Eq + Display,
    {
        println!("Now creating new texture with dimensions {w}x{h} and ID {key}");
        let tex = self.loader.create(w, h);
        if let Some(_) = self.table.get(&key) {
            let resource = Rc::new(RefCell::new(tex));
            self.table.insert(key, Rc::clone(&resource));
            Ok(resource.clone())
        } else {
            let resource = Rc::new(RefCell::new(tex));
            self.table.insert(key, Rc::clone(&resource));
            Ok(resource.clone())
        }
    }

    // Generics magic to allow a HashMap to use String as a key
    // while allowing it to use &str for gets
    pub fn load<D>(&mut self, key: K, details: &D) -> Result<Rc<RefCell<R>>, String>
    where
        L: ResourceLoader<'l, R, Args = D>,
        D: ?Sized + 'l,
        K: 'l + Hash + Eq,
    {
        if let Some(_) = self.table.get(&key) {
            Err(String::from("Value already exists"))
        } else {
            let resource = Rc::new(RefCell::new(self.loader.load(details)?));
            self.table.insert(key, Rc::clone(&resource));
            Ok(resource.clone())
        }
    }

    pub fn get(&self, key: &K) -> Option<Rc<RefCell<R>>> {
        self.table.get(key).cloned()
    }
}

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
    pub fn default() -> Self {
        FontChar {
            ch: 0 as char,
            bbox: Rect::new(0, 0, 0, 0),
            _ax: 0,
            _ay: 0,
            bl: 0,
            bt: 0,
        }
    }
    pub fn new(ch: char, bbox: Rect, _ax: u32, _ay: u32, bl: i32, bt: i32) -> Self {
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

        if info.ch.is_whitespace() {
            Rect::new(x, y, info._ax, self.glyph_height)
        } else {
            Rect::new(
                x + center_dist,
                y + baseline_dist,
                info.bbox.width(),
                info.bbox.height(),
            )
        }
    }

    /// Get the position of the character in the texture atlas
    pub fn get_char(&self, char: usize) -> Result<Rc<FontChar>, ()> {
        if let Some(info) = self.char_lookup.get(&char) {
            Ok(info.clone())
        } else {
            Err(())
        }
    }

    pub fn get_string<T: Into<String>>(&self, str: T) -> Result<Vec<Rc<FontChar>>, ()> {
        let str: String = str.into();
        let mut vec = Vec::<Rc<FontChar>>::with_capacity(str.len());
        for ch in str.chars() {
            vec.push(self.get_char(ch as usize)?);
        }
        Ok(vec)
    }
}
