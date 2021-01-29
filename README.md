# mdns

[![Build Status](https://travis-ci.org/dylanmckay/mdns.svg?branch=master)](https://travis-ci.org/dylanmckay/mdns)
[![crates.io](https://img.shields.io/crates/v/mdns.svg)](https://crates.io/crates/mdns)
[![MIT license](https://img.shields.io/github/license/mashape/apistatus.svg)]()

[Documentation](https://docs.rs/mdns)

An multicast DNS client in Rust.

Error logging is handled with the `log` library.

[Wikipedia](https://en.wikipedia.org/wiki/Multicast_DNS)

## Example

Find IP addresses for all Chromecasts on the local network.

```rust
use futures_util::{pin_mut, stream::StreamExt};
use mdns::{Error, Record, RecordKind};
use std::{net::IpAddr, time::Duration};


const SERVICE_NAME: &'static str = "_googlecast._tcp.local";

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Iterate through responses from each Cast device, asking for new devices every 15s
    let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
    pin_mut!(stream);

    while let Some(Ok(response)) = stream.next().await {
        let addr = response.records()
                           .filter_map(self::to_ip_addr)
                           .next();

        if let Some(addr) = addr {
            println!("found cast device at {}", addr);
        } else {
            println!("cast device does not advertise address");
        }
    }

    Ok(())
}

fn to_ip_addr(record: &Record) -> Option<IpAddr> {
    match record.kind {
        RecordKind::A(addr) => Some(addr.into()),
        RecordKind::AAAA(addr) => Some(addr.into()),
        _ => None,
    }
}
```
