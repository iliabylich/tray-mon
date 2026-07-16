use crate::{
    dbus_queue::DBusQueue,
    infallible_property::InfalliblePropertyGetAndSubscribe,
    tray::{Service, icon::TrayIcon, icons_cache::IconsCache},
};
use anyhow::{Context, Result};
use icon::Icons;
use mini_sansio_dbus::{
    EncodeError, IncomingMessage, OutgoingQueue,
    messages::sni_host::{
        EventArgs, IconName, IconPixmap, IconPixmapBytes, ItemsPropertiesUpdatedSignal,
        ItemsPropertiesUpdatedSubscribe, ItemsPropertiesUpdatedUnsubscribe, LayoutUpdatedSignal,
        LayoutUpdatedSubscribe, LayoutUpdatedUnsubscribe, Menu, NewIconSignal, NewIconSubscribe,
        NewIconUnsubscribe,
    },
    messaging::{DBusEncode, reply_handler::ReplyHandler},
};
use std::rc::Rc;

mod state;
use state::State;

mod get_layout;
use get_layout::{GetLayout, TrayItem, VecOfTrayItems};
use tray_mon::{Elements, Event, FixedSizeString};

pub struct App {
    service: Service,

    menu_prop: InfalliblePropertyGetAndSubscribe<Menu<String>>,
    icon_name_prop: InfalliblePropertyGetAndSubscribe<IconName<String>>,
    icon_pixmap_prop: InfalliblePropertyGetAndSubscribe<IconPixmap<String, VecOfU8>>,

    get_layout: Option<ReplyHandler<GetLayout>>,

    menu_path: String,
    state: State,

    icon: Option<Rc<TrayIcon>>,
    menu: Option<Elements>,
}

struct VecOfU8(Vec<u8>);
impl IconPixmapBytes for VecOfU8 {
    fn new() -> Self {
        Self(vec![])
    }

    fn push(&mut self, v: u8) {
        self.0.push(v);
    }
}

impl App {
    pub(crate) const fn new(service: Service) -> Self {
        Self {
            service,

            menu_prop: InfalliblePropertyGetAndSubscribe::new(),
            icon_name_prop: InfalliblePropertyGetAndSubscribe::new(),
            icon_pixmap_prop: InfalliblePropertyGetAndSubscribe::new(),

            get_layout: None,

            menu_path: String::new(),
            state: State::Nothing,

            icon: None,
            menu: None,
        }
    }

    fn schedule_request_props(&mut self, q: &mut DBusQueue) {
        self.menu_prop
            .get(Menu::new(self.service.name().to_string()), q);
        self.icon_name_prop
            .get(IconName::new(self.service.name().to_string()), q);
        self.icon_pixmap_prop
            .get(IconPixmap::new(self.service.name().to_string()), q);
    }

    pub(crate) fn init(&mut self, q: &mut DBusQueue) -> Result<()> {
        let mut bytes = [0; 1_024];
        let buf = NewIconSubscribe::encode(self.service.name(), &mut bytes)?;
        q.push_raw(buf);

        self.menu_prop
            .get_and_subscribe(Menu::new(self.service.name().to_string()), q);
        self.icon_name_prop
            .get_and_subscribe(IconName::new(self.service.name().to_string()), q);
        self.icon_pixmap_prop
            .get_and_subscribe(IconPixmap::new(self.service.name().to_string()), q);

        Ok(())
    }

    pub(crate) fn reset(&mut self, q: &mut DBusQueue) -> Result<()> {
        let mut bytes = [0; 1_024];

        let buf = NewIconUnsubscribe::encode(self.service.name(), &mut bytes)?;
        q.push_raw(buf);

        self.menu_prop.unsubscribe(q);
        self.icon_name_prop.unsubscribe(q);
        self.icon_pixmap_prop.unsubscribe(q);

        let buf = LayoutUpdatedUnsubscribe::encode(
            (self.service.name(), self.menu_path.as_str()),
            &mut bytes,
        )?;
        q.push_raw(buf);

        let buf = ItemsPropertiesUpdatedUnsubscribe::encode(
            (self.service.name(), self.menu_path.as_str()),
            &mut bytes,
        )?;
        q.push_raw(buf);

        Ok(())
    }

    fn schedule_get_layout(&mut self, q: &mut DBusQueue) -> Result<()> {
        let mut buf = [0; 1_024];
        let buf = GetLayout::encode((self.service.name(), self.menu_path.as_str()), &mut buf)?;

        let message = GetLayout::new(self.service.name().to_string());
        let serial = q.push_raw(buf);

        self.get_layout = Some(ReplyHandler::new(serial, message));
        Ok(())
    }

    fn on_menu_received(&mut self, menu_path: &str, q: &mut DBusQueue) -> Result<()> {
        if !self.menu_path.is_empty() {
            return Ok(());
        }

        self.menu_path = String::from(menu_path);
        let mut bytes = [0; 1_024];
        self.schedule_get_layout(q)?;

        let buf = LayoutUpdatedSubscribe::encode((self.service.name(), menu_path), &mut bytes)?;
        q.push_raw(buf);

        let buf =
            ItemsPropertiesUpdatedSubscribe::encode((self.service.name(), menu_path), &mut bytes)?;
        q.push_raw(buf);
        Ok(())
    }

    pub(crate) fn handle(
        &mut self,
        message: IncomingMessage<'_>,
        q: &mut DBusQueue,
        icons: &Icons,
        icons_cache: &mut IconsCache,
    ) -> Result<Option<Event>> {
        if message.sender != Some(self.service.raw_address()) {
            return Ok(None);
        }

        if let Some(menu_path) = self.menu_prop.handle_reply_or_signal(message, q) {
            log::info!(target: "Tray", "Received menu {:?} - {menu_path:?}", self.service);
            self.on_menu_received(menu_path, q)?;
            return Ok(None);
        }

        if let Some(name_or_path) = self.icon_name_prop.handle_reply_or_signal(message, q) {
            if name_or_path.is_empty() {
                return Ok(None);
            }
            log::info!(target: "Tray", "Received icon name {:?}", self.service);
            let icon = icons_cache.new_icon_name(name_or_path, icons)?;
            self.icon = Some(Rc::clone(&icon));
            let event = self.state.on_icon_received(self.service.name(), icon)?;
            return Ok(event);
        }

        if let Some((width, height, VecOfU8(bytes))) =
            self.icon_pixmap_prop.handle_reply_or_signal(message, q)
        {
            log::info!(target: "Tray", "Received icon pixmap {:?}", self.service);
            let width = u32::try_from(width).context("negative icon pixmap width")?;
            let height = u32::try_from(height).context("negative icon pixmap height")?;
            let icon = icons_cache.new_icon_pixmap(width, height, bytes)?;
            self.icon = Some(Rc::clone(&icon));
            let event = self.state.on_icon_received(self.service.name(), icon)?;
            return Ok(event);
        }

        if let Some(get_layout) = &self.get_layout
            && let Some(VecOfTrayItems(layout)) = get_layout.handle(message)?
        {
            log::info!(target: "Tray", "Got layout");
            let elements = TrayItem::flatten(layout)?;
            self.menu = Some(elements);
            let event = self
                .state
                .on_layout_receieved(self.service.name(), &elements)?;
            return Ok(event);
        }

        if NewIconSignal::matches(message, self.service.raw_address()) {
            log::info!(target: "Tray", "Received NewIcon signal");
            self.schedule_request_props(q);
            return Ok(None);
        }

        if LayoutUpdatedSignal::matches(
            message,
            self.service.raw_address(),
            self.menu_path.as_str(),
        ) {
            log::info!(target: "Tray", "Received LayoutUpdated signal");
            self.schedule_get_layout(q)?;
            return Ok(None);
        }

        if ItemsPropertiesUpdatedSignal::matches(
            message,
            self.service.raw_address(),
            self.menu_path.as_str(),
        ) {
            log::info!(target: "Tray", "Received ItemsPropertiesUpdated signal");
            self.schedule_get_layout(q)?;
            return Ok(None);
        }

        Ok(None)
    }

    pub(crate) fn trigger(&self, id: i32, q: &mut DBusQueue) -> Result<(), EncodeError> {
        let now = chrono::Utc::now().timestamp();
        let timestamp = u32::try_from(now).map_err(|_| EncodeError::ValueTooLong)?;
        let args = EventArgs {
            id,
            timestamp,
            destination: self.service.name(),
            path: self.menu_path.as_str(),
        };
        let mut buf = [0; 1_024];
        let buf = mini_sansio_dbus::messages::sni_host::Event::encode(args, &mut buf)?;
        q.push_raw(buf);

        Ok(())
    }

    pub(crate) fn serialize(&self) -> Result<Option<Event>> {
        let Some(icon) = self.icon.as_ref() else {
            return Ok(None);
        };
        let Some(menu) = self.menu else {
            return Ok(None);
        };
        Ok(Some(Event::AppAdded {
            service: FixedSizeString::new(self.service.name())?,
            icon: FixedSizeString::new(icon.filepath())?,
            menu,
        }))
    }
}
