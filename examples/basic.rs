extern crate mdns;

fn main() {
    mdns::run().expect("error while running mDNS discovery")
}
