#![forbid(unsafe_code)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications,
    deprecated_in_future,
    unused_lifetimes,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic,
    clippy::nursery
)]
#![expect(
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    clippy::mut_mut,
    clippy::large_enum_variant
)]

use anyhow::{Result, ensure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Event {
    AppAdded {
        service: FixedSizeString,
        icon: FixedSizeString,
        menu: Elements,
    },
    AppRemoved {
        service: FixedSizeString,
    },
    MenuUpdated {
        service: FixedSizeString,
        menu: Elements,
    },
    IconUpdated {
        service: FixedSizeString,
        icon: FixedSizeString,
    },
}
impl Event {
    pub const SERIALIZED_BYTESIZE: usize = 1
        + FixedSizeString::SERIALIZED_BYTESIZE
        + FixedSizeString::SERIALIZED_BYTESIZE
        + Elements::SERIALIZED_BYTESIZE;

    #[must_use]
    pub fn serialize(self) -> [u8; Self::SERIALIZED_BYTESIZE] {
        let mut out = [0; _];
        let mut buf = &mut out[..];

        match self {
            Self::AppAdded {
                service,
                icon,
                menu,
            } => {
                write_u8(&mut buf, 1);
                write_fixed_size_string(&mut buf, &service);
                write_fixed_size_string(&mut buf, &icon);
                write_menu(&mut buf, &menu);
            }
            Self::AppRemoved { service } => {
                write_u8(&mut buf, 2);
                write_fixed_size_string(&mut buf, &service);
            }
            Self::MenuUpdated { service, menu } => {
                write_u8(&mut buf, 3);
                write_fixed_size_string(&mut buf, &service);
                write_menu(&mut buf, &menu);
            }
            Self::IconUpdated { service, icon } => {
                write_u8(&mut buf, 4);
                write_fixed_size_string(&mut buf, &service);
                write_fixed_size_string(&mut buf, &icon);
            }
        }

        out
    }

    pub fn deserialize(buf: [u8; Self::SERIALIZED_BYTESIZE]) -> Self {
        let mut buf = &buf[..];

        match read_u8(&mut buf) {
            1 => {
                let service = read_fixed_size_string(&mut buf);
                let icon = read_fixed_size_string(&mut buf);
                let menu = read_menu(&mut buf);
                Self::AppAdded {
                    service,
                    icon,
                    menu,
                }
            }
            2 => {
                let service = read_fixed_size_string(&mut buf);
                Self::AppRemoved { service }
            }
            3 => {
                let service = read_fixed_size_string(&mut buf);
                let menu = read_menu(&mut buf);
                Self::MenuUpdated { service, menu }
            }
            4 => {
                let service = read_fixed_size_string(&mut buf);
                let icon = read_fixed_size_string(&mut buf);
                Self::IconUpdated { service, icon }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elements(pub [Element; Self::ITEMS_COUNT]);

impl Elements {
    pub const ITEMS_COUNT: usize = 20;
    pub const SERIALIZED_BYTESIZE: usize = Self::ITEMS_COUNT * Element::SERIALIZED_BYTESIZE;

    #[cfg(test)]
    fn random() -> Self {
        let mut elements = [Element::None; _];
        for element in &mut elements {
            *element = Element::random();
        }
        Self(elements)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Element {
    Regular {
        id: i32,
        label: Label,
    },
    Disabled {
        id: i32,
        label: Label,
    },
    Checkbox {
        id: i32,
        label: Label,
        checked: bool,
    },
    Radio {
        id: i32,
        label: Label,
        selected: bool,
    },
    Nested {
        id: i32,
        label: Label,
        child_start_idx: u64,
        child_end_idx: u64,
    },
    Section {
        child_start_idx: u64,
        child_end_idx: u64,
    },
    None,
}
impl Element {
    pub const SERIALIZED_BYTESIZE: usize =
        1 + size_of::<i32>() + Label::SERIALIZED_BYTESIZE + size_of::<u64>() * 2;

    #[must_use]
    pub fn serialize(self) -> [u8; Self::SERIALIZED_BYTESIZE] {
        let mut out = [0; _];
        let mut buf = &mut out[..];

        match self {
            Self::Regular { id, label } => {
                write_u8(&mut buf, 1);
                write_i32(&mut buf, id);
                write_label(&mut buf, &label);
            }
            Self::Disabled { id, label } => {
                write_u8(&mut buf, 2);
                write_i32(&mut buf, id);
                write_label(&mut buf, &label);
            }
            Self::Checkbox { id, label, checked } => {
                write_u8(&mut buf, 3);
                write_i32(&mut buf, id);
                write_u8(&mut buf, u8::from(checked));
                write_label(&mut buf, &label);
            }
            Self::Radio {
                id,
                label,
                selected,
            } => {
                write_u8(&mut buf, 4);
                write_i32(&mut buf, id);
                write_u8(&mut buf, u8::from(selected));
                write_label(&mut buf, &label);
            }
            Self::Nested {
                id,
                label,
                child_start_idx,
                child_end_idx,
            } => {
                write_u8(&mut buf, 5);
                write_i32(&mut buf, id);
                write_u64(&mut buf, child_start_idx);
                write_u64(&mut buf, child_end_idx);
                write_label(&mut buf, &label);
            }
            Self::Section {
                child_start_idx,
                child_end_idx,
            } => {
                write_u8(&mut buf, 6);
                write_u64(&mut buf, child_start_idx);
                write_u64(&mut buf, child_end_idx);
            }
            Self::None => {}
        }

        let _ = buf;
        out
    }

    pub fn deserialize(buf: [u8; Self::SERIALIZED_BYTESIZE]) -> Self {
        let mut buf = &buf[..];

        match read_u8(&mut buf) {
            0 => Self::None,

            1 => {
                let id = read_i32(&mut buf);
                let label = read_label(&mut buf);
                Self::Regular { id, label }
            }

            2 => {
                let id = read_i32(&mut buf);
                let label = read_label(&mut buf);
                Self::Disabled { id, label }
            }

            3 => {
                let id = read_i32(&mut buf);
                let checked = read_u8(&mut buf);
                let label = read_label(&mut buf);
                Self::Checkbox {
                    id,
                    label,
                    checked: checked == 1,
                }
            }

            4 => {
                let id = read_i32(&mut buf);
                let selected = read_u8(&mut buf);
                let label = read_label(&mut buf);
                Self::Radio {
                    id,
                    label,
                    selected: selected == 1,
                }
            }

            5 => {
                let id = read_i32(&mut buf);
                let child_start_idx = read_u64(&mut buf);
                let child_end_idx = read_u64(&mut buf);
                let label = read_label(&mut buf);
                Self::Nested {
                    id,
                    label,
                    child_start_idx,
                    child_end_idx,
                }
            }

            6 => {
                let child_start_idx = read_u64(&mut buf);
                let child_end_idx = read_u64(&mut buf);
                Self::Section {
                    child_start_idx,
                    child_end_idx,
                }
            }

            _ => unreachable!(),
        }
    }

    #[cfg(test)]
    fn random() -> Self {
        match rand::random_range(1..=6) {
            1 => Self::Regular {
                id: rand::random(),
                label: Label::random(),
            },
            2 => Self::Disabled {
                id: rand::random(),
                label: Label::random(),
            },
            3 => Self::Checkbox {
                id: rand::random(),
                label: Label::random(),
                checked: rand::random(),
            },
            4 => Self::Radio {
                id: rand::random(),
                label: Label::random(),
                selected: rand::random(),
            },
            5 => Self::Nested {
                id: rand::random(),
                label: Label::random(),
                child_start_idx: rand::random(),
                child_end_idx: rand::random(),
            },
            6 => Self::Section {
                child_start_idx: rand::random(),
                child_end_idx: rand::random(),
            },

            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct Label {
    buf: [u8; Self::STR_BYTESIZE],
    len: u32,
}
impl Label {
    pub const STR_BYTESIZE: usize = 50;
    pub const SERIALIZED_BYTESIZE: usize = Self::STR_BYTESIZE + 4;

    #[must_use]
    pub fn serialize(self) -> [u8; Self::SERIALIZED_BYTESIZE] {
        let mut buf = [0; _];
        buf[0..4].copy_from_slice(&self.len.to_be_bytes());
        buf[4..4 + Self::STR_BYTESIZE].copy_from_slice(&self.buf);
        buf
    }

    pub fn deserialize(buf: [u8; Self::SERIALIZED_BYTESIZE]) -> Self {
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let mut s = [0; _];
        s.copy_from_slice(&buf[4..]);
        Self { len, buf: s }
    }

    pub fn new(s: &str) -> Self {
        let mut out = [0; Self::STR_BYTESIZE];
        let mut bytes_written = 0;
        for c in s.chars() {
            let len = c.len_utf8();
            assert!(len <= 4);
            if bytes_written + len >= Self::STR_BYTESIZE {
                break;
            }

            let mut buf = [0; 4];
            c.encode_utf8(&mut buf);
            let buf = &buf[..len];

            let start = bytes_written;
            let end = start + len;
            out[start..end].copy_from_slice(buf);

            bytes_written += buf.len();
        }
        Self {
            buf: out,
            len: bytes_written as u32,
        }
    }

    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        std::str::from_utf8(&self.buf[..self.len as usize])
    }

    #[cfg(test)]
    fn random() -> Self {
        let len: usize = rand::random_range(0..Self::STR_BYTESIZE);
        let mut buf = [0; _];
        for i in 0..len {
            buf[i] = rand::random_range(b'a'..=b'z');
        }
        Self {
            buf,
            len: len as u32,
        }
    }
}
impl core::fmt::Debug for Label {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct FixedSizeString {
    buf: [u8; Self::STR_BYTESIZE],
    len: u32,
}
impl FixedSizeString {
    pub const STR_BYTESIZE: usize = 256;
    pub const SERIALIZED_BYTESIZE: usize = Self::STR_BYTESIZE + 4;

    #[must_use]
    pub fn serialize(self) -> [u8; Self::SERIALIZED_BYTESIZE] {
        let mut buf = [0; _];
        buf[0..4].copy_from_slice(&self.len.to_be_bytes());
        buf[4..4 + Self::STR_BYTESIZE].copy_from_slice(&self.buf);
        buf
    }

    pub fn deserialize(buf: [u8; Self::SERIALIZED_BYTESIZE]) -> Self {
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let mut s = [0; _];
        s.copy_from_slice(&buf[4..]);
        Self { len, buf: s }
    }

    pub fn new(s: impl AsRef<[u8]>) -> Result<Self> {
        let s = s.as_ref();
        ensure!(s.len() < Self::STR_BYTESIZE);

        let mut out = [0; Self::STR_BYTESIZE];
        out[0..s.len()].copy_from_slice(s);
        Ok(Self {
            buf: out,
            len: s.len() as u32,
        })
    }

    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        std::str::from_utf8(&self.buf[..self.len as usize])
    }

    #[cfg(test)]
    fn random() -> Self {
        let len: usize = rand::random_range(0..Self::STR_BYTESIZE);
        let mut buf = [0; _];
        for i in 0..len {
            buf[i] = rand::random_range(b'a'..=b'z');
        }
        Self {
            buf,
            len: len as u32,
        }
    }
}
impl core::fmt::Debug for FixedSizeString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?}",
            String::from_utf8_lossy(&self.buf[..self.len as usize])
        )
    }
}

fn write_u8(buf: &mut &mut [u8], n: u8) {
    let (head, tail) = std::mem::take(buf).split_at_mut(1);
    *buf = tail;
    head[0] = n;
}
fn write_i32(buf: &mut &mut [u8], n: i32) {
    let (head, rest) = std::mem::take(buf).split_at_mut(4);
    *buf = rest;
    head.copy_from_slice(&n.to_be_bytes());
}
fn write_label(buf: &mut &mut [u8], text: &Label) {
    let (head, tail) = std::mem::take(buf).split_at_mut(Label::SERIALIZED_BYTESIZE);
    *buf = tail;
    head.copy_from_slice(&text.serialize());
}
fn write_fixed_size_string(buf: &mut &mut [u8], text: &FixedSizeString) {
    let (head, tail) = std::mem::take(buf).split_at_mut(FixedSizeString::SERIALIZED_BYTESIZE);
    *buf = tail;
    head.copy_from_slice(&text.serialize());
}
fn write_u64(buf: &mut &mut [u8], n: u64) {
    let (head, tail) = std::mem::take(buf).split_at_mut(8);
    *buf = tail;
    head.copy_from_slice(&n.to_be_bytes());
}
fn write_element(buf: &mut &mut [u8], element: Element) {
    let (head, tail) = std::mem::take(buf).split_at_mut(Element::SERIALIZED_BYTESIZE);
    *buf = tail;
    head.copy_from_slice(&element.serialize());
}
fn write_menu(buf: &mut &mut [u8], elements: &Elements) {
    for element in elements.0 {
        write_element(buf, element);
    }
}

fn read_u8(buf: &mut &[u8]) -> u8 {
    let n = buf[0];
    *buf = &buf[1..];
    n
}
fn read_i32(buf: &mut &[u8]) -> i32 {
    let n = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    *buf = &buf[4..];
    n
}
fn read_label(buf: &mut &[u8]) -> Label {
    let mut textbuf = [0; _];
    textbuf.copy_from_slice(&buf[0..Label::SERIALIZED_BYTESIZE]);
    let text = Label::deserialize(textbuf);
    *buf = &buf[Label::SERIALIZED_BYTESIZE..];
    text
}
fn read_fixed_size_string(buf: &mut &[u8]) -> FixedSizeString {
    let mut textbuf = [0; _];
    textbuf.copy_from_slice(&buf[0..FixedSizeString::SERIALIZED_BYTESIZE]);
    let text = FixedSizeString::deserialize(textbuf);
    *buf = &buf[FixedSizeString::SERIALIZED_BYTESIZE..];
    text
}
fn read_u64(buf: &mut &[u8]) -> u64 {
    let n = u64::from_be_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ]);
    *buf = &buf[8..];
    n
}
fn read_element(buf: &mut &[u8]) -> Element {
    let mut elementbuf = [0; _];
    elementbuf.copy_from_slice(&buf[0..Element::SERIALIZED_BYTESIZE]);
    let text = Element::deserialize(elementbuf);
    *buf = &buf[Element::SERIALIZED_BYTESIZE..];
    text
}
fn read_menu(buf: &mut &[u8]) -> Elements {
    let mut elements = [Element::None; Elements::ITEMS_COUNT];
    for element in &mut elements {
        *element = read_element(buf);
    }
    Elements(elements)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialize_label() {
        let label = Label::random();
        assert_eq!(Label::deserialize(label.serialize()), label);
    }

    #[test]
    fn test_serialize_s() {
        let s = FixedSizeString::random();
        assert_eq!(FixedSizeString::deserialize(s.serialize()), s);
    }

    #[test]
    fn test_serialize_element_regular() {
        let e = Element::Regular {
            id: 42,
            label: Label::random(),
        };
        assert_eq!(Element::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_element_disabled() {
        let e = Element::Disabled {
            id: 42,
            label: Label::random(),
        };
        assert_eq!(Element::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_element_checkbox() {
        let e = Element::Checkbox {
            id: 42,
            label: Label::random(),
            checked: true,
        };
        assert_eq!(Element::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_element_radio() {
        let e = Element::Radio {
            id: 42,
            label: Label::random(),
            selected: true,
        };
        assert_eq!(Element::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_element_nested() {
        let e = Element::Nested {
            id: 42,
            label: Label::random(),
            child_start_idx: 42,
            child_end_idx: 142,
        };
        assert_eq!(Element::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_element_section() {
        let e = Element::Section {
            child_start_idx: 142,
            child_end_idx: 242,
        };
        assert_eq!(Element::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_app_added() {
        let e = Event::AppAdded {
            service: FixedSizeString::random(),
            icon: FixedSizeString::random(),
            menu: Elements::random(),
        };
        assert_eq!(Event::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_app_removed() {
        let e = Event::AppRemoved {
            service: FixedSizeString::random(),
        };
        assert_eq!(Event::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_menu_updated() {
        let e = Event::MenuUpdated {
            service: FixedSizeString::random(),
            menu: Elements::random(),
        };
        assert_eq!(Event::deserialize(e.serialize()), e);
    }

    #[test]
    fn test_serialize_icon_updated() {
        let e = Event::IconUpdated {
            service: FixedSizeString::random(),
            icon: FixedSizeString::random(),
        };
        assert_eq!(Event::deserialize(e.serialize()), e);
    }
}
