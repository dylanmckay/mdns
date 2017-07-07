use {mDNS, Error, Response};
use std::time::{SystemTime, Duration};

use io;

/// Runs discovery with a callback that can be notifed of responses.
pub fn discover_with<F>(service_name: &str,
                        duration: Option<Duration>,
                        mut f: F) -> Result<(), Error>
    where F: FnMut(Response) -> Result<(), Error> {
    let mut io = io::Io::new()?;
    let mut mdns = mDNS::new(service_name, &mut io)?;

    let finish_at = duration.map(|duration| SystemTime::now() + duration);

    loop {
        let poll_timeout = finish_at.map(|finish_at| {
            finish_at.duration_since(SystemTime::now()).unwrap()
        });

        io.poll(&mut mdns, poll_timeout)?;

        for response in mdns.responses() {
            f(response)?;
        }

        if let Some(finish_at) = finish_at {
            if SystemTime::now() >= finish_at {
                break;
            }
        }
    }
    Ok(())
}
