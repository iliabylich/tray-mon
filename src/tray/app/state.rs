use crate::tray::{TrayIcon, name_cache::NameCache};
use anyhow::Result;
use std::rc::Rc;
use tray_mon::{Elements, Event, FixedSizeString};

#[derive(Default)]
pub enum State {
    #[default]
    Nothing,
    OnlyIcon(Rc<TrayIcon>),
    OnlyLayout(Box<Elements>),
    All,
}

impl State {
    pub(crate) fn on_icon_received(
        &mut self,
        service: &str,
        new_icon: Rc<TrayIcon>,
        name_cache: &mut NameCache,
    ) -> Result<Option<Event>> {
        match core::mem::take(self) {
            Self::Nothing => {
                *self = Self::OnlyIcon(new_icon);
                Ok(None)
            }
            Self::OnlyIcon(_icon) => {
                *self = Self::OnlyIcon(new_icon);
                Ok(None)
            }
            Self::OnlyLayout(layout) => {
                *self = Self::All;
                Ok(Some(Event::AppAdded {
                    service: name_cache.add(service),
                    icon: FixedSizeString::new(new_icon.filepath())?,
                    menu: *layout,
                }))
            }
            Self::All => {
                *self = Self::All;
                Ok(Some(Event::IconUpdated {
                    service: name_cache.add(service),
                    icon: FixedSizeString::new(new_icon.filepath())?,
                }))
            }
        }
    }

    pub(crate) fn on_layout_receieved(
        &mut self,
        service: &str,
        new_layout: &Elements,
        name_cache: &mut NameCache,
    ) -> Result<Option<Event>> {
        match core::mem::take(self) {
            Self::Nothing => {
                *self = Self::OnlyLayout(Box::new(*new_layout));
                Ok(None)
            }
            Self::OnlyIcon(icon) => {
                *self = Self::All;
                Ok(Some(Event::AppAdded {
                    service: name_cache.add(service),
                    icon: FixedSizeString::new(icon.filepath())?,
                    menu: *new_layout,
                }))
            }
            Self::OnlyLayout(_layout) => {
                *self = Self::OnlyLayout(Box::new(*new_layout));
                Ok(None)
            }
            Self::All => {
                *self = Self::All;
                Ok(Some(Event::MenuUpdated {
                    service: name_cache.add(service),
                    menu: *new_layout,
                }))
            }
        }
    }
}
