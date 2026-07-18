use anyhow::{Result, bail};
use mini_sansio_dbus::messages::sni_host::{
    GetLayout as GetLayoutIface, GetLayoutItem, GetLayoutList,
};
use std::collections::VecDeque;
use tray_mon::{Element, Elements, Label};

#[derive(Debug, Clone)]
pub enum TrayItem {
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
        children: Vec<Self>,
    },
    Section {
        children: Vec<Self>,
    },
}

pub type GetLayout = GetLayoutIface<VecOfTrayItems, TrayItem, String>;

#[derive(Default)]
pub struct VecOfTrayItems(pub(crate) Vec<TrayItem>);

impl GetLayoutList for VecOfTrayItems {
    type Item = TrayItem;

    fn new() -> Self {
        Self(vec![])
    }

    fn push(&mut self, item: Self::Item) {
        self.0.push(item);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl GetLayoutItem for TrayItem {
    type List = VecOfTrayItems;

    fn new_section(children: Self::List) -> Self {
        Self::Section {
            children: children.0,
        }
    }

    fn new_nested(id: i32, _service: &str, label: &str, children: Self::List) -> Self {
        Self::Nested {
            id,
            label: Label::new(label),
            children: children.0,
        }
    }

    fn new_disabled(id: i32, _service: &str, label: &str) -> Self {
        Self::Disabled {
            id,
            label: Label::new(label),
        }
    }

    fn new_checkbox(id: i32, _service: &str, label: &str, checked: bool) -> Self {
        Self::Checkbox {
            id,
            label: Label::new(label),
            checked,
        }
    }

    fn new_radio(id: i32, _service: &str, label: &str, selected: bool) -> Self {
        Self::Radio {
            id,
            label: Label::new(label),
            selected,
        }
    }

    fn new_regular(id: i32, _service: &str, label: &str) -> Self {
        Self::Regular {
            id,
            label: Label::new(label),
        }
    }
}

impl TrayItem {
    pub(crate) fn flatten(items: Vec<Self>) -> Result<Elements> {
        struct QueueItem {
            nodes: Vec<TrayItem>,
            parent_idx: Option<usize>,
        }
        let mut queue = VecDeque::new();
        let mut flat = vec![];
        queue.push_back(QueueItem {
            nodes: items,
            parent_idx: None,
        });
        while let Some(QueueItem { nodes, parent_idx }) = queue.pop_front() {
            let start_idx = flat.len();
            for node in nodes {
                let root = parent_idx.is_none();
                let element = node.as_element_ignoring_children(root);
                flat.push(element);

                match node {
                    Self::Regular { .. }
                    | Self::Disabled { .. }
                    | Self::Checkbox { .. }
                    | Self::Radio { .. } => {}

                    Self::Nested { children, .. } | Self::Section { children } => {
                        queue.push_back(QueueItem {
                            nodes: children,
                            parent_idx: Some(flat.len() - 1),
                        });
                    }
                }
            }
            let end_idx = flat.len();

            if let Some(parent_idx) = parent_idx {
                match &mut flat[parent_idx] {
                    parent @ (Element::Regular { .. }
                    | Element::Disabled { .. }
                    | Element::Checkbox { .. }
                    | Element::Radio { .. }
                    | Element::None) => {
                        bail!(
                            "something is wrong with tree structure, parent {parent:?} can't have children"
                        )
                    }

                    Element::Nested {
                        child_start_idx,
                        child_end_idx,
                        ..
                    }
                    | Element::Section {
                        child_start_idx,
                        child_end_idx,
                        ..
                    } => {
                        *child_start_idx = start_idx as u64;
                        *child_end_idx = end_idx as u64;
                    }
                }
            }
        }

        while flat.len() < Elements::ITEMS_COUNT {
            flat.push(Element::None);
        }
        Ok(Elements(flat.try_into().map_err(|vec: Vec<Element>| {
            anyhow::anyhow!(
                "too many tray items, max: {}, given: {}",
                Elements::ITEMS_COUNT,
                vec.len()
            )
        })?))
    }

    const fn as_element_ignoring_children(&self, root: bool) -> Element {
        match self {
            Self::Regular { id, label } => Element::Regular {
                id: *id,
                root,
                label: *label,
            },
            Self::Disabled { id, label } => Element::Disabled {
                id: *id,
                root,
                label: *label,
            },
            Self::Checkbox { id, label, checked } => Element::Checkbox {
                id: *id,
                root,
                label: *label,
                checked: *checked,
            },
            Self::Radio {
                id,
                label,
                selected,
            } => Element::Radio {
                id: *id,
                root,
                label: *label,
                selected: *selected,
            },
            Self::Nested {
                id,
                label,
                children: _children,
            } => Element::Nested {
                id: *id,
                root,
                label: *label,
                child_start_idx: 0,
                child_end_idx: 0,
            },
            Self::Section {
                children: _children,
            } => Element::Section {
                root,
                child_start_idx: 0,
                child_end_idx: 0,
            },
        }
    }
}
