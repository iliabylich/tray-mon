use std::collections::HashMap;

pub struct NameCache {
    name_to_id: HashMap<String, u32>,
    next_id: u32,
}

impl NameCache {
    pub(crate) fn new() -> Self {
        Self {
            name_to_id: HashMap::new(),
            next_id: 1,
        }
    }

    pub(crate) fn add(&mut self, name: &str) -> u32 {
        if let Some(existing) = self.name_to_id.get(name) {
            return *existing;
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.name_to_id.insert(name.to_string(), id);
        id
    }

    pub(crate) fn resolve(&self, id: u32) -> Option<String> {
        self.name_to_id
            .iter()
            .find(|(_name, existing_id)| **existing_id == id)
            .map(|(name, _)| name.clone())
    }
}
