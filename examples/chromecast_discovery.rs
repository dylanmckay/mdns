extern crate mdns;

use mdns::{Record, RecordKind};
use std::{net::IpAddr, time::Duration};

const SERVICE_NAME: &'static str = "_googlecast._tcp.local";

fn main() {
    for response in mdns::discover::all_timeout(SERVICE_NAME, Duration::from_secs(5)).unwrap() {
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

