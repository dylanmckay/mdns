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
//! use futures_util::{pin_mut, stream::StreamExt};
//! use mdns::{Error, Record, RecordKind};
//! use std::{net::IpAddr, time::Duration};
//!
//! /// The hostname of the devices we are searching for.
//! /// Every Chromecast will respond to the service name in this example.
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! #[cfg_attr(feature = "runtime-async-std", async_std::main)]
//! #[cfg_attr(feature = "runtime-tokio", tokio::main)]
//! async fn main() -> Result<(), Error> {
//!     // Iterate through responses from each Cast device, asking for new devices every 15s
//!     let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
//!     pin_mut!(stream);
//!
//!     while let Some(Ok(response)) = stream.next().await {
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
//!
//!     Ok(())
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

#[cfg(all(feature = "runtime-async-std", feature = "runtime-tokio"))]
compile_error!("\"runtime-async-std\" and \"runtime-tokio\" cannot be enabled simultaneously");

#[cfg(not(any(feature = "runtime-async-std", feature = "runtime-tokio")))]
compile_error!("At least one runtime (\"runtime-async-std\" or \"runtime-tokio\") cargo feature must be enabled");

pub use self::errors::Error;
pub use self::response::{Record, RecordKind, Response};

pub mod discover;
pub mod resolve;

mod runtime;

mod errors;
mod mdns;
mod response;

pub use self::mdns::mDNSListener;
