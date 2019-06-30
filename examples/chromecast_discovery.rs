extern crate futures;
extern crate mdns;
extern crate tokio;

use futures::{Future, Stream};
use mdns::{Record, RecordKind};
use std::{net::IpAddr, time::Duration};

const SERVICE_NAME: &'static str = "_http._tcp.local";

fn main() {
    tokio::run(
        mdns::discover::all(SERVICE_NAME, Duration::from_secs(5))
            .unwrap()
            .for_each(|response| {
                let addr = response.records().filter_map(self::to_ip_addr).next();

                if let Some(addr) = addr {
                    println!("found cast device at {}", addr);
                } else {
                    println!("cast device does not advertise address");
                }

                Ok(())
            })
            .map_err(|_| ()),
    );
}

fn to_ip_addr(record: &Record) -> Option<IpAddr> {
    match record.kind {
        RecordKind::A(addr) => Some(addr.into()),
        RecordKind::AAAA(addr) => Some(addr.into()),
        _ => None,
    }
}
