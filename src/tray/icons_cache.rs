use crate::tray::icon::TrayIcon;
use anyhow::Result;
use icon::Icons;
use std::{
    collections::{HashMap, hash_map::Entry},
    rc::Rc,
};

pub struct IconsCache {
    cache: HashMap<blake3::Hash, Rc<TrayIcon>>,
}

impl IconsCache {
    pub(crate) fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub(crate) fn new_icon_name(&mut self, icon_name: &str, icons: &Icons) -> Result<Rc<TrayIcon>> {
        self.new_with(
            {
                let mut hasher = blake3::Hasher::new();
                hasher.update(icon_name.as_bytes());
                hasher.finalize()
            },
            move || TrayIcon::new_icon_name(icon_name, icons),
        )
    }

    pub(crate) fn new_icon_pixmap(
        &mut self,
        width: u32,
        height: u32,
        bytes: Vec<u8>,
    ) -> Result<Rc<TrayIcon>> {
        self.new_with(
            {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&width.to_be_bytes());
                hasher.update(&height.to_be_bytes());
                hasher.update(&bytes);
                hasher.finalize()
            },
            move || TrayIcon::new_icon_pixmap(width, height, bytes),
        )
    }

    fn new_with(
        &mut self,
        key: blake3::Hash,
        make: impl FnOnce() -> Result<TrayIcon>,
    ) -> Result<Rc<TrayIcon>> {
        match self.cache.entry(key) {
            Entry::Occupied(occupied) => Ok(Rc::clone(occupied.get())),
            Entry::Vacant(vacant) => {
                let icon = Rc::new(make()?);
                vacant.insert(Rc::clone(&icon));
                Ok(icon)
            }
        }
    }
}
