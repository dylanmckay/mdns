use crate::{Error, Response};

use std::{sync::Arc, time::Duration};

use crate::mdns::mDNSSender;
use async_std::future::TimeoutError;
use futures_core::{Future, Stream};
use futures_util::{future::ready, stream::select, StreamExt};

pub use async_std::net::UdpSocket as AsyncUdpSocket;

pub fn discovery_listen(
    ignore_empty: bool,
    service_name: String,
    response_stream: impl Stream<Item = Result<Response, Error>>,
    sender: mDNSSender,
    request_interval: Duration,
) -> impl Stream<Item = Result<Response, Error>> {
    let response_stream = response_stream.map(super::StreamResult::Response);

    let interval_stream = async_std::stream::interval(request_interval)
        // I don't like the double clone, I can't find a prettier way to do this
        .map(move |_| {
            let mut sender = sender.clone();
            async_std::task::spawn(async move {
                let _ = sender.send_request().await;
            });
            super::StreamResult::Interval
        });

    let stream = select(response_stream, interval_stream);
    stream
        .filter_map(|stream_result| async {
            match stream_result {
                super::StreamResult::Interval => None,
                super::StreamResult::Response(res) => Some(res),
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

pub fn make_async_socket(socket: std::net::UdpSocket) -> Result<Arc<AsyncUdpSocket>, Error> {
    Ok(Arc::new(AsyncUdpSocket::from(socket)))
}

pub async fn timeout<F, T>(future: F, timeout: Duration) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    async_std::future::timeout(timeout, future).await
}
