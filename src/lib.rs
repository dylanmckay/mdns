#![recursion_limit = "1024"]

pub use self::response::{Response, Record, RecordKind};
pub use self::errors::{Error, ErrorKind, ResultExt};

pub mod discover;

mod mdns;
mod response;
mod errors;
mod io;

use self::mdns::mDNS;
use self::io::Io;

extern crate mio;
extern crate dns_parser as dns;
extern crate net2;
extern crate ifaces;

#[macro_use]
extern crate error_chain;
