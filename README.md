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

use std::time::Duration;

fn main() {
    let duration = Duration::from_secs(5);

    mdns::discover("_googlecast._tcp.local", Some(duration), |response| {
        let addresses = response.records().filter_map(|record| {
            if let mdns::RecordKind::A(addr) = record.kind { Some(addr) } else { None }
        });

        for address in addresses {
            println!("found Chromecast on {}", address);
        }
    }).expect("error while performing Chromecast discovery");
}
```
