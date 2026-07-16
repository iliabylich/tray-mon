use crate::{dbus_queue::DBusQueue, tray::icons_cache::IconsCache};
use ::icon::Icons;
use anyhow::Result;
use app::App;
use icon::TrayIcon;
use mini_sansio_dbus::{
    IncomingMessage, OutgoingQueue,
    messages::org_freedesktop_dbus::{NameOwnerChangedSignal, NameOwnerChangedSubscribe},
    messaging::DBusEncode as _,
};
use service::Service;
use status_notifier_watcher::StatusNotifierWatcher;
use tray_mon::{Event, FixedSizeString};

mod app;
mod icon;
mod icons_cache;
mod service;
mod status_notifier_watcher;

pub struct Tray {
    registry: Vec<(Service, App)>,
    icons: Icons,
    icons_cache: IconsCache,
}

impl Tray {
    pub(crate) fn new(q: &mut DBusQueue) -> Result<Self> {
        Self::init(q)?;
        let icons = Icons::new();
        Ok(Self {
            registry: vec![],
            icons,
            icons_cache: IconsCache::new(),
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
            self.registry.retain(|(s, _)| s != &service);
            self.registry.push((service, tray_app));
            return Ok(None);
        }

        if let Some(service) = NameOwnerChangedSignal::handle(message)? {
            let Some(idx) = self
                .registry
                .iter()
                .position(|(s, _)| s.name() == service || s.raw_address() == service)
            else {
                return Ok(None);
            };

            let (_, mut tray_app) = self.registry.remove(idx);

            log::info!(target: "Tray", "Removed {service}");
            tray_app.reset(q)?;
            return Ok(Some(Event::AppRemoved {
                service: FixedSizeString::new(service)?,
            }));
        }

        for (service, app) in &mut self.registry {
            if let Some(event) = app.handle(message, q, &self.icons, &mut self.icons_cache)? {
                let service = service.name();

                log::info!(target: "Tray", "{service:?} {event:#?}");
                return Ok(Some(event));
            }
        }

        Ok(None)
    }

    fn try_trigger(&self, service: &str, id: i32, q: &mut DBusQueue) -> Result<()> {
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

    #[expect(dead_code)]
    pub(crate) fn trigger(&self, service: &str, id: i32, q: &mut DBusQueue) {
        if let Err(err) = self.try_trigger(service, id, q) {
            log::error!("{err:?}");
        }
    }

    pub(crate) fn serialize(&self) -> Vec<Vec<u8>> {
        let mut blobs = vec![];
        for (_, app) in &self.registry {
            if let Ok(Some(blob)) = app.serialize() {
                blobs.push(blob.serialize().to_vec());
            }
        }
        blobs
    }
}
