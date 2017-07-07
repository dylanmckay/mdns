extern crate mdns;

const SERVICE_NAME: &'static str = "_googlecast._tcp.local";

fn main() {
    for response in mdns::discover::all(SERVICE_NAME).unwrap() {
        let response = response.unwrap();

        println!("response: {:?}", response);
    }
}
