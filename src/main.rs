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

use anyhow::{Context, Result};
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
        RunMode::Dev => {
            let xdg_runtime_dir =
                std::env::var("XDG_RUNTIME_DIR").context("no $XDG_RUNTIME_DIR")?;
            let path = format!("{xdg_runtime_dir}/tray-mon-dev.sock");
            Runtime::new_with_socket_path(&path, service)
        }
    }?;

    loop {
        rt.poll()?;
    }
}
