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
//! #[async_std::main]
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

use crate::mdns::{mDNSSender, mdns_interface};
use futures_core::Stream;
use futures_util::{future::ready, stream::select, StreamExt};
use std::net::Ipv4Addr;

/// A multicast DNS discovery request.
///
/// This represents a single lookup of a single service name.
///
/// This object can be iterated over to yield the received mDNS responses.
pub struct Discovery {
    service_name: String,

    mdns_sender: mDNSSender,
    mdns_listener: mDNSListener,

    /// Whether we should ignore empty responses.
    ignore_empty: bool,

    /// The interval we should send mDNS queries.
    send_request_interval: Duration,
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
    let service_name = service_name.as_ref().to_string();
    let (mdns_listener, mdns_sender) = mdns_interface(service_name.clone(), interface_addr)?;

    Ok(Discovery {
        service_name,
        mdns_sender,
        mdns_listener,
        ignore_empty: true,
        send_request_interval: mdns_query_interval,
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

    pub fn listen(self) -> impl Stream<Item = Result<Response, Error>> {
        let ignore_empty = self.ignore_empty;
        let service_name = self.service_name;
        let response_stream = self.mdns_listener.listen().map(StreamResult::Response);
        let sender = self.mdns_sender.clone();
        let mut imm_sender = sender.clone();

        let interval_stream = async_std::stream::interval(self.send_request_interval)
            // I don't like the double clone, I can't find a prettier way to do this
            .map(move |_| {
                let mut sender = sender.clone();
                async_std::task::spawn(async move {
                    let _ = sender.send_request().await;
                });
                StreamResult::Interval
            });

        // Send first request immediately
        async_std::task::spawn(async move {
            let _ = imm_sender.send_request().await;
        });

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
                    Ok(response) => {
                        (!response.is_empty() || !ignore_empty)
                            && response
                                .answers
                                .iter()
                                .any(|record| record.name == service_name)
                    }
                    Err(_) => true,
                })
            })
    }
}

enum StreamResult {
    Interval,
    Response(Result<Response, Error>),
}
