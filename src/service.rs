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

impl Service for TrayMonService {
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

    fn state_as_serialized_bufs(&self) -> impl Iterator<Item = Vec<u8>> {
        self.tray.serialize().into_iter()
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
