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

use anyhow::Result;
use pollsendd::Runtime;

mod args;
mod dbus;
mod dbus_queue;
mod infallible_property;
mod service;
mod tray;

use args::{Args, RunMode};
use service::TrayMonService;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .write_style(env_logger::WriteStyle::Always)
        .init();

    let args = Args::parse();
    log::trace!("Running with args {args:?}");

    let service = TrayMonService::new()?;

    let mut rt = match args.mode {
        RunMode::Systemd => Runtime::new_with_systemd_socket(service),
        RunMode::Dev => Runtime::new_with_socket_path("tray-mon-dev.sock", service),
    }?;

    loop {
        rt.poll()?;
    }
}
