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

use crate::{io, mDNS, Error, Response};

use std::collections::VecDeque;
use std::time::{SystemTime, Duration};

const POLL_DURATION_MS: u64 = 10;
const TIME_BETWEEN_SOLICITATIONS_MS: u64 = 3_000;

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
    /// When we last asked clients for responses.
    last_solicitation_sent_at: Option<SystemTime>,
    /// Whether we should ignore empty responses.
    ignore_empty: bool,
}

/// Gets an iterator over all responses for a given service.
pub fn all<S>(service_name: S) -> Result<Discovery, Error> where S: AsRef<str> {
    all_ext(service_name, None)
}

/// Gets an iterator over all responses for a given service.
pub fn all_timeout<S>(service_name: S,
                      timeout: Duration)
    -> Result<Discovery, Error> where S: AsRef<str> {
    all_ext(service_name, Some(timeout))
}

fn all_ext<S>(service_name: S,
              timeout: Option<Duration>)
    -> Result<Discovery, Error>
    where S: AsRef<str> {
    let mut io = io::Io::new()?;
    let mdns = mDNS::new(service_name.as_ref(), &mut io)?;

    Ok(Discovery {
        io,
        mdns,
        responses: VecDeque::new(),
        finish_at: timeout.map(|timeout| SystemTime::now() + timeout),
        last_solicitation_sent_at: None,
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
        let poll_timeout = Duration::from_millis(POLL_DURATION_MS);

        loop {
            self.io.poll(&mut self.mdns, Some(poll_timeout))?;

            // Only ask for more responses if we haven't timed out yet.
            if !self.timeout_surpassed() {
                // Only ask for more responses if we haven't done so recently; don't flood the
                // network (sorry @ruuda).
                let should_solicit = match self.last_solicitation_sent_at {
                    Some(last_time) => SystemTime::now() >= last_time + Duration::from_millis(TIME_BETWEEN_SOLICITATIONS_MS),
                    None => true, // first solicitation.
                };

                if should_solicit {
                    for client_token in self.mdns.client_tokens() {
                        self.mdns.send_if_ready(client_token)?;
                    }

                    self.last_solicitation_sent_at = Some(SystemTime::now());
                }
            }

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
                std::thread::yield_now();
                continue;
            }

            // We have at least one response, or the timeout has run out.
            self.responses.extend(responses.into_iter());

            if self.timeout_surpassed() {
                break;
            }
        }

        Ok(())
    }
}

impl Iterator for Discovery {
    type Item = Result<Response, Error>;

    fn next(&mut self) -> Option<Result<Response, Error>> {
        if !self.timeout_surpassed() {
            if let Err(e) = self.poll() {
                return Some(Err(e));
            }
        }

        self.responses.pop_front().map(Ok)
    }
}

