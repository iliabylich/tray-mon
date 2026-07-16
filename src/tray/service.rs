#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Service {
    raw_address: String,
    name: String,
}

impl Service {
    pub(crate) const fn new(raw_address: String, name: String) -> Self {
        Self { raw_address, name }
    }

    pub(crate) const fn raw_address(&self) -> &str {
        self.raw_address.as_str()
    }

    pub(crate) const fn name(&self) -> &str {
        self.name.as_str()
    }
}
