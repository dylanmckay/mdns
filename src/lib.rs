//! [Multicast DNS](https://en.wikipedia.org/wiki/Multicast_DNS) library with built-in networking.
//!
//! This crate can be used to discover mDNS devices that are listening
//! on a network.
//!
//! # Basic usage
//!
//! This example finds all [Chromecast](https://en.wikipedia.org/wiki/Chromecast) devices on the
//! same LAN as the executing computer.
//!
//! Once the devices are discovered, they respond with standard DNS records, with a few minor
//! low-level protocol differences.
//!
//! The only Chromecast-specific piece of code here is the `SERVICE_NAME`. In order to discover
//! other types of devices, simply change the service name to the one your device uses.
//!
//! This example obtains the IP addresses of the cast devices by looking up `A`/`AAAA` records.
//!
//! ```rust,no_run
//! extern crate mdns;
//!
//! use mdns::{Record, RecordKind};
//! use std::net::IpAddr;
//!
//! /// The hostname of the devices we are searching for.
//! /// Every Chromecast will respond to the service name in this example.
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! fn main() {
//!     // Iterate through responses from each Cast device.
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
