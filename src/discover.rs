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

use std::collections::VecDeque;
use std::time::{SystemTime, Duration};

use io;

/// A multicast DNS discovery request.
///
/// This represents a single lookup of a single service name.
///
/// This object can be iterated over to yield the received mDNS responses.
pub struct Discovery {
    io: io::Io,
    mdns: mDNS,

    /// The responses we have received but not iterated over yet.
    responses: VecDeque<Response>,

    /// An optional timeout value which represents when we will stop
    /// checking for responses.
    finish_at: Option<SystemTime>,
    /// Whether we should ignore empty responses.
    ignore_empty: bool,
}

/// Gets an iterator over all responses for a given service.
pub fn all<S>(service_name: S) -> Result<Discovery, Error> where S: AsRef<str> {
    let mut io = io::Io::new()?;
    let mdns = mDNS::new(service_name.as_ref(), &mut io)?;

    Ok(Discovery {
        io,
        mdns,
        responses: VecDeque::new(),
        finish_at: None,
        ignore_empty: true,
    })
}

impl Discovery {
    /// Sets a timeout for discovery.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.finish_at = Some(SystemTime::now() + duration);
        self
    }

    /// Sets whether or not we should ignore empty responses.
    ///
    /// Defaults to `true`.
    pub fn ignore_empty(mut self, ignore: bool) -> Self {
        self.ignore_empty = ignore;
        self
    }

    /// Checks if the timeout has been surpassed.
    fn timeout_surpassed(&self) -> bool {
        self.finish_at.map(|finish_at| SystemTime::now() >= finish_at).unwrap_or(false)
    }

    fn poll(&mut self) -> Result<(), Error> {
        loop {
            let poll_timeout = self.finish_at.map(|finish_at| {
                finish_at.duration_since(SystemTime::now()).unwrap()
            });

            self.io.poll(&mut self.mdns, poll_timeout)?;

            let ignore_empty = self.ignore_empty;
            let responses: Vec<_> =
                self.mdns.responses()
                         .filter(|r| if ignore_empty { !r.is_empty() } else { true })
                         .collect();

            // We can get writable events which will exit the poll loop before
            // we even read a response. For our purposes, we want to read
            // at least one response in this method so long as the timeout hasn't passed.
            //
            // That way our callers can be sure that there is at least one response
            // if the timeout hasn't passed.
            if responses.is_empty() && !self.timeout_surpassed() {
                continue;
            } else {
                // We have at least one response, or the timeout has run out.
                self.responses.extend(responses.into_iter());
                break;
            }
        }

        Ok(())
    }
}

impl Iterator for Discovery {
    type Item = Result<Response, Error>;

    fn next(&mut self) -> Option<Result<Response, Error>> {
        if self.timeout_surpassed() { return None };

        if let Err(e) = self.poll() {
            return Some(Err(e));
        }

        self.responses.pop_front().map(Ok)
    }
}

