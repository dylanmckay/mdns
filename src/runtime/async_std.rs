use crate::Error;

use async_std::future::TimeoutError;
use futures_core::{Future, Stream};
use std::{sync::Arc, time::Duration};

pub use async_std::net::UdpSocket as AsyncUdpSocket;
pub use async_std::task::spawn;

pub fn create_interval_stream(request_interval: Duration) -> impl Stream<Item = ()> {
    async_std::stream::interval(request_interval)
}

pub fn make_async_socket(socket: std::net::UdpSocket) -> Result<Arc<AsyncUdpSocket>, Error> {
    Ok(Arc::new(AsyncUdpSocket::from(socket)))
}

pub async fn timeout<F, T>(timeout: Duration, future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    async_std::future::timeout(timeout, future).await
}
