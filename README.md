# mdns

[![Build Status](https://travis-ci.org/dylanmckay/mdns.svg?branch=master)](https://travis-ci.org/dylanmckay/mdns)
[![crates.io](https://img.shields.io/crates/v/mdns.svg)](https://crates.io/crates/mdns)
[![MIT license](https://img.shields.io/github/license/mashape/apistatus.svg)]()

[Documentation](https://docs.rs/mdns)

An multicast DNS client in Rust.

[Wikipedia](https://en.wikipedia.org/wiki/Multicast_DNS)

## Example

Find IP addresses for all Chromecasts on the local network.

```rust
extern crate mdns;

use mdns::{Record, RecordKind};
use std::net::IpAddr;

const SERVICE_NAME: &'static str = "_googlecast._tcp.local";

fn main() {
    for response in mdns::discover::all(SERVICE_NAME).unwrap() {
        let response = response.unwrap();

        let addr = response.records()
                           .filter_map(self::to_ip_addr)
                           .next();

        if let Some(addr) = addr {
            println!("found cast device at {}", addr);
        } else {
            println!("cast device does not advertise address");
        }
    }
}

fn to_ip_addr(record: &Record) -> Option<IpAddr> {
    match record.kind {
        RecordKind::A(addr) => Some(addr.into()),
        RecordKind::AAAA(addr) => Some(addr.into()),
        _ => None,
    }
}
```
