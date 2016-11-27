#![recursion_limit = "1024"]

pub use self::mdns::mDNS;
pub use self::response::{Response, Record, RecordKind};
pub use self::errors::{Error, ErrorKind};
pub use self::discover::discover;

pub mod mdns;
pub mod response;
pub mod errors;
pub mod discover;
mod io;

extern crate mio;
extern crate dns_parser as dns;
extern crate net2;

#[macro_use]
extern crate error_chain;

pub fn run() -> Result<(), Error> {
    let mut io = io::Io::new()?;

    let mut mdns = mDNS::new("_googlecast._tcp.local")?;
    io.register(&mut mdns)?;

    loop {
        io.poll(&mut mdns, None)?;
    }
}
