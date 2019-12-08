//! Utilities for discovering devices on the LAN.
//!
//! Examples
//!
//! ```rust,no_run
//! use futures_util::{pin_mut, stream::StreamExt};
//! use mdns::{Error, Record, RecordKind};
//! use std::time::Duration;
//!
//! const SERVICE_NAME: &'static str = "_googlecast._tcp.local";
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let stream = mdns::discover::all(SERVICE_NAME, Duration::from_secs(15))?.listen();
//!     pin_mut!(stream);
//!
//!     while let Some(Ok(response)) = stream.next().await {
//!         println!("{:?}", response);
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::{mDNSListener, Error, Response};

use std::time::Duration;

use tokio::time;

use crate::mdns::{mDNSSender, mdns_interface};
use async_stream::stream;
use futures_core::Stream;
use futures_util::{future::ready, stream::select, StreamExt};
use std::net::Ipv4Addr;

/// A multicast DNS discovery request.
///
/// This represents a single lookup of a single service name.
///
/// This object can be iterated over to yield the received mDNS responses.
pub struct Discovery {
    mdns_sender: mDNSSender,
    mdns_listener: mDNSListener,

    /// Whether we should ignore empty responses.
    ignore_empty: bool,

    /// The interval we should send mDNS queries.
    send_request_interval: time::Interval,
}

/// Gets an iterator over all responses for a given service on all interfaces.
pub fn all<S>(service_name: S, mdns_query_interval: Duration) -> Result<Discovery, Error>
where
    S: AsRef<str>,
{
    interface(service_name, mdns_query_interval, Ipv4Addr::new(0, 0, 0, 0))
}

/// Gets an iterator over all responses for a given service on a given interface.
pub fn interface<S>(
    service_name: S,
    mdns_query_interval: Duration,
    interface_addr: Ipv4Addr,
) -> Result<Discovery, Error>
where
    S: AsRef<str>,
{
    let (mdns_listener, mdns_sender) =
        mdns_interface(service_name.as_ref().to_string(), interface_addr)?;

    Ok(Discovery {
        mdns_sender,
        mdns_listener,
        ignore_empty: true,
        send_request_interval: time::interval(mdns_query_interval),
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

    fn interval_send(
        mut interval: time::Interval,
        mut sender: mDNSSender,
    ) -> impl Stream<Item = ()> {
        stream! {
            loop {
                interval.tick().await;
                sender.send_request().await;

                yield;
            }
        }
    }

    pub fn listen(self) -> impl Stream<Item = Result<Response, Error>> {
        let ignore_empty = self.ignore_empty;
        let response_stream = self
            .mdns_listener
            .listen()
            .map(|res| StreamResult::Response(res));

        let interval_stream = Self::interval_send(self.send_request_interval, self.mdns_sender)
            .map(|_| StreamResult::Interval);

        let stream = select(response_stream, interval_stream);
        stream
            .filter_map(|stream_result| {
                async {
                    match stream_result {
                        StreamResult::Interval => None,
                        StreamResult::Response(res) => Some(res),
                    }
                }
            })
            .filter(move |res| {
                ready(match res {
                    Ok(response) => !response.is_empty() || !ignore_empty,
                    Err(_) => true,
                })
            })
    }
}

enum StreamResult {
    Interval,
    Response(Result<Response, Error>),
}
