#![recursion_limit = "1024"]

pub use self::mdns::mDNS;
pub use self::response::{Response, Record, RecordKind};
pub use self::errors::{Error, ErrorKind};

pub mod mdns;
pub mod response;
pub mod errors;

extern crate mio;
extern crate dns_parser as dns;
extern crate net2;

#[macro_use]
extern crate error_chain;

use mio::*;

const SERVER: Token = Token(0);

pub fn run() -> Result<(), Error> {
    let mut mdns = mDNS::new("_googlecast._tcp.local")?;

    let poll = Poll::new()?;
    mdns.register_io(&poll, SERVER)?;

    // Create storage for events
    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            assert_eq!(event.token(), SERVER);
            mdns.handle_io(event)?;
        }
    }
}
