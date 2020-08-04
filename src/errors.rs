use err_derive::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "_0")]
    Io(#[error(source)] std::io::Error),
    #[error(display = "_0")]
    Dns(#[error(source)] dns_parser::Error),
    #[error(display = "_0")]
    TimeoutError(#[error(source)] async_std::future::TimeoutError),
}
