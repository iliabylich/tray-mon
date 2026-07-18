use anyhow::{Context as _, Result};
use rustix::net::{AddressFamily, SocketAddrUnix, SocketType};
use tray_mon::Event;

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: cli /path/to/socket");
    std::process::exit(1);
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        eprintln!("no path given");
        print_usage_and_exit()
    });

    let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)
        .context("socket() failed")?;

    let addr = SocketAddrUnix::new(path).context("can't build UNIX socket address")?;
    rustix::net::connect(&fd, &addr).context("connect() failed")?;

    rustix::io::write(&fd, &[0, 0, 0, 1, 0, 0, 0, 2]).unwrap();

    loop {
        let mut buf = [0; Event::SERIALIZED_BYTESIZE];
        let len = rustix::io::read(&fd, &mut buf)?;
        assert!(len == buf.len());
        let event = Event::deserialize(buf);
        println!("{event:#?}");
    }
}
