//! Utilities for discovering devices on the LAN.
//!
//! Examples
//!
//! ```rust,no_run
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! fn main() {
//!     for response in mdns::discover::all(SERVICE_NAME).unwrap() {
//!         let response = response.unwrap();
//!
//!         println!("{:?}", response);
//!     }
//! }
//! ```

use {mDNS, Error, Response};

use std::time::{Duration, Instant};

use tokio_timer::Interval;

use futures::{try_ready, Async::Ready, Poll, Stream};

/// A multicast DNS discovery request.
///
/// This represents a single lookup of a single service name.
///
/// This object can be iterated over to yield the received mDNS responses.
pub struct Discovery {
    mdns: mDNS,

    /// Whether we should ignore empty responses.
    ignore_empty: bool,

    /// The interval we should send mDNS queries.
    send_request_interval: Interval,
}

/// Gets an iterator over all responses for a given service.
pub fn all<S>(service_name: S, mdns_query_interval: Duration) -> Result<Discovery, Error>
where
    S: AsRef<str>,
{
    let mdns = mDNS::new(service_name.as_ref())?;

    Ok(Discovery {
        mdns,
        ignore_empty: true,
        send_request_interval: Interval::new(Instant::now(), mdns_query_interval),
    })
}

impl Discovery {
    /// Sets whether or not we should ignore empty responses.
    ///
    /// Defaults to `true`.
    pub fn ignore_empty(mut self, ignore: bool) -> Self {
        self.ignore_empty = ignore;
        self
    }
}

impl Stream for Discovery {
    type Item = Response;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self
            .send_request_interval
            .poll()
            .map(|r| r.is_ready())
            .unwrap_or(false)
        {
            self.mdns.send_request()?;
        }

        loop {
            let resp = match try_ready!(self.mdns.poll()) {
                Some(response) => response,
                None => return Ok(Ready(None)),
            };

            if !resp.is_empty() || !self.ignore_empty {
                return Ok(Ready(Some(resp)));
            }
        }
    }
}
