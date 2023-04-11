use std::{cell::RefCell, collections::HashMap, fmt::Display, hash::Hash, rc::Rc};

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
