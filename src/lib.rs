//! Multicast DNS library.
//!
//! # Basic usage
//!
//! ```rust,no_run
//! extern crate mdns;
//!
//! use mdns::{Record, RecordKind};
//! use std::net::IpAddr;
//!
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! fn main() {
//!     for response in mdns::discover::all(SERVICE_NAME).unwrap() {
//!         let response = response.unwrap();
//!
//!         let addr = response.records()
//!                            .filter_map(self::to_ip_addr)
//!                            .next();
//!
//!         if let Some(addr) = addr {
//!             println!("found cast device at {}", addr);
//!         } else {
//!             println!("cast device does not advertise address");
//!         }
//!     }
//! }
//!
//! fn to_ip_addr(record: &Record) -> Option<IpAddr> {
//!     match record.kind {
//!         RecordKind::A(addr) => Some(addr.into()),
//!         RecordKind::AAAA(addr) => Some(addr.into()),
//!         _ => None,
//!     }
//! }
//! ```

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
