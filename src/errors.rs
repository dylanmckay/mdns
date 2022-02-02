use err_derive::Error;

#[cfg(feature = "runtime-async-std")]
pub use async_std::future::TimeoutError;

#[cfg(feature = "runtime-tokio")]
pub use tokio::time::error::Elapsed as TimeoutError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "_0")]
    Io(#[error(source)] std::io::Error),
    #[error(display = "_0")]
    Dns(#[error(source)] dns_parser::Error),
    #[error(display = "_0")]
    TimeoutError(#[error(source)] TimeoutError),
}
