use crate::{dbus::DBus, dbus_queue::DBusQueue, tray::Tray};
use anyhow::Result;
use pollsendd::Service;
use rustix::event::{PollFd, PollFlags};
use std::os::fd::AsRawFd;

pub struct TrayMonService {
    queue: DBusQueue,
    tray: Tray,
    dbus: DBus,
    readbuf: Vec<u8>,
}

impl TrayMonService {
    pub(crate) fn new() -> Result<Self> {
        let mut queue = DBusQueue::new()?;
        let tray = Tray::new(&mut queue)?;

        let dbus = DBus::new()?;
        let readbuf = vec![0; 400 * 1_024];

        Ok(Self {
            queue,
            tray,
            dbus,
            readbuf,
        })
    }
}

impl Service<8> for TrayMonService {
    type Error = TrayMonServiceError;

    fn pollfds(&mut self) -> Result<impl Iterator<Item = PollFd<'_>>, Self::Error> {
        Ok(std::iter::once(
            self.dbus.as_pollfd(&mut self.readbuf, &self.queue)?,
        ))
    }

    fn owns_fd(&self, fd: i32) -> bool {
        self.dbus.as_raw_fd() == fd
    }

    fn on_readable(
        &mut self,
        _fd: i32,
        mut broadcast: impl FnMut(&[u8]),
    ) -> Result<(), Self::Error> {
        if let Some(message) = self.dbus.read(&mut self.readbuf, &self.queue)?
            && let Some(event) = self.tray.handle(message, &mut self.queue)
        {
            (broadcast)(&event.serialize());
        }
        Ok(())
    }

    fn on_writable(&mut self, _fd: i32, _broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error> {
        self.dbus.write(&mut self.readbuf, &mut self.queue)?;
        Ok(())
    }

    fn on_poll_error(&mut self, _fd: i32, revents: PollFlags) -> Result<(), Self::Error> {
        Err(anyhow::anyhow!("DBus poll error: got revents {revents:?}").into())
    }

    fn on_client_connected(&mut self, mut write: impl FnMut(Vec<u8>)) -> Result<(), Self::Error> {
        for buf in self.tray.serialize() {
            (write)(buf);
        }
        Ok(())
    }

    fn on_client_request(&mut self, buf: [u8; 8]) -> Result<(), Self::Error> {
        let service_id = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let id = i32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        log::error!("Request: service_id={service_id} id={id}");
        self.tray.trigger(service_id, id, &mut self.queue);
        Ok(())
    }
}

pub struct TrayMonServiceError(anyhow::Error);
impl std::fmt::Display for TrayMonServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::fmt::Debug for TrayMonServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl core::error::Error for TrayMonServiceError {}
impl From<anyhow::Error> for TrayMonServiceError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}
