use futures_util::{pin_mut, stream::StreamExt};
use mdns::Error;
use std::time::Duration;

const SERVICE_NAME: &str = "_hue._tcp.local";

#[cfg_attr(feature = "runtime-async-std", async_std::main)]
#[cfg_attr(feature = "runtime-tokio", tokio::main)]
async fn main() -> Result<(), Error> {
    let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
    pin_mut!(stream);
    while let Some(Ok(response)) = stream.next().await {
        let addr = response.ip_addr();

        if let Some(addr) = addr {
            println!("found Hue bridge at {}", addr);
        } else {
            println!("cast device does not advertise address");
        }
    }
    Ok(())
}
