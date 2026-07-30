use std::collections::HashMap;

use crate::dbus_queue::DBusQueue;
use ::icon::Icons;
use anyhow::Result;
use app::App;
use icon::TrayIcon;
use icons_cache::IconsCache;
use mini_sansio_dbus::{
    IncomingMessage, OutgoingQueue,
    messages::org_freedesktop_dbus::{NameOwnerChangedSignal, NameOwnerChangedSubscribe},
    messaging::DBusEncode as _,
};
use name_cache::NameCache;
use service::Service;
use status_notifier_watcher::StatusNotifierWatcher;
use tray_mon::Event;

mod app;
mod icon;
mod icons_cache;
mod name_cache;
mod service;
mod status_notifier_watcher;

pub struct Tray {
    registry: HashMap<Service, App>,
    icons: Icons,
    icons_cache: IconsCache,
    name_cache: NameCache,
}

impl Tray {
    pub(crate) fn new(q: &mut DBusQueue) -> Result<Self> {
        Self::init(q)?;
        let icons = Icons::new();
        Ok(Self {
            registry: HashMap::new(),
            icons,
            icons_cache: IconsCache::new(),
            name_cache: NameCache::new(),
        })
    }

    fn init(q: &mut DBusQueue) -> Result<()> {
        StatusNotifierWatcher::request_ksni_name(q)?;

        let mut buf = [0; 1_024];
        let buf = NameOwnerChangedSubscribe::encode((), &mut buf)?;
        q.push_raw(buf);
        Ok(())
    }

    pub(crate) fn handle(
        &mut self,
        message: IncomingMessage<'_>,
        q: &mut DBusQueue,
    ) -> Option<Event> {
        match self.try_handle(message, q) {
            Ok(event) => event,
            Err(err) => {
                log::error!("{err:?}");
                None
            }
        }
    }

    fn try_handle(
        &mut self,
        message: IncomingMessage<'_>,
        q: &mut DBusQueue,
    ) -> Result<Option<Event>> {
        if let Some(service) = StatusNotifierWatcher::handle_incoming_request(message, q)? {
            log::info!(target: "Tray", "Added {service:?}");
            let mut tray_app = App::new(service.clone());
            tray_app.init(q)?;
            self.registry.insert(service, tray_app);
            return Ok(None);
        }

        if let Some(NameOwnerChangedSignal::Disappeared { name, owner }) =
            NameOwnerChangedSignal::handle(message)?
        {
            let Some(key) = self
                .registry
                .keys()
                .find(|s| s.name() == name || s.raw_address() == owner)
                .cloned()
            else {
                return Ok(None);
            };

            let Some(mut tray_app) = self.registry.remove(&key) else {
                return Ok(None);
            };

            log::info!(target: "Tray", "Removed {name}/{owner}");
            tray_app.reset(q)?;
            return Ok(Some(Event::AppRemoved {
                service: self.name_cache.add(name),
            }));
        }

        for (service, app) in &mut self.registry {
            if let Some(event) = app.handle(
                message,
                q,
                &self.icons,
                &mut self.icons_cache,
                &mut self.name_cache,
            )? {
                let service = service.name();

                log::info!(target: "Tray", "{service:?} {event:#?}");
                return Ok(Some(event));
            }
        }

        Ok(None)
    }

    fn try_trigger(&self, service_id: u32, id: i32, q: &mut DBusQueue) -> Result<()> {
        let Some(service) = self.name_cache.resolve(service_id) else {
            log::warn!("Unknown service id {service_id}");
            return Ok(());
        };

        let Some((_, tray_app)) = self
            .registry
            .iter()
            .find(|(s, _)| s.name() == service || s.raw_address() == service)
        else {
            log::info!(target: "Tray", "service {service} doesn't exist");
            return Ok(());
        };

        tray_app.trigger(id, q)?;
        Ok(())
    }

    pub(crate) fn trigger(&self, service_id: u32, id: i32, q: &mut DBusQueue) {
        if let Err(err) = self.try_trigger(service_id, id, q) {
            log::error!("{err:?}");
        }
    }

    pub(crate) fn serialize(&mut self) -> Vec<Vec<u8>> {
        let mut blobs = vec![];
        for app in self.registry.values() {
            if let Ok(Some(blob)) = app.serialize(&mut self.name_cache) {
                blobs.push(blob.serialize().to_vec());
            }
        }
        blobs
    }
}
