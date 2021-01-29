use futures_util::{pin_mut, stream::StreamExt};
use mdns::Error;
use std::time::Duration;

const SERVICE_NAME: &'static str = "_http._tcp.local";

#[async_std::main]
async fn main() -> Result<(), Error> {
    let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
    pin_mut!(stream);
    while let Some(Ok(response)) = stream.next().await {
        let addr = response.ip_addr();

        if let Some(addr) = addr {
            println!("found cast device at {}", addr);
        } else {
            println!("cast device does not advertise address");
        }
    }
    Ok(())
}
