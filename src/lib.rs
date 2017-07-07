//! Multicast DNS library.
//!
//! # Basic usage
//!
//! ```rust,no_run
//!
//! extern crate mdns;
//!
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! fn main() {
//!     for response in mdns::discover::all(SERVICE_NAME).unwrap() {
//!         let response = response.unwrap();
//!
//!         println!("response: {:?}", response);
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
