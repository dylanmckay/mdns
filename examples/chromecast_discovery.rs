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
