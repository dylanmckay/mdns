use mdns::Error;
use std::time::Duration;

const SERVICE_NAME: &str = "_http._tcp.local";
const HOSTS: [&str; 2] = ["server1._http._tcp.local", "server2._http._tcp.local"];

#[cfg_attr(feature = "runtime-async-std", async_std::main)]
#[cfg_attr(feature = "runtime-tokio", tokio::main)]
async fn main() -> Result<(), Error> {
    let responses = mdns::resolve::multiple(SERVICE_NAME, &HOSTS, Duration::from_secs(15)).await?;

    for response in responses {
        if let (Some(host), Some(ip)) = (response.hostname(), response.ip_addr()) {
            println!("found host {} at {}", host, ip)
        }
    }

    Ok(())
}
