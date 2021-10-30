use crate::{Error, Response};

use std::{io, net::Ipv4Addr};

use async_stream::try_stream;
use futures_core::Stream;
use std::sync::Arc;
use async_std::net::UdpSocket;

use net2::UdpSocketExt;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;
use std::net::SocketAddr;

/// The IP address for the mDNS multicast socket.
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MULTICAST_PORT: u16 = 5353;

pub fn mdns_interface(
    service_name: String,
    interface_addr: Ipv4Addr,
) -> Result<(mDNSListener, mDNSSender), Error> {
    let socket = create_socket()?;

    socket.set_multicast_loop_v4(false)?;
    socket.join_multicast_v4(&MULTICAST_ADDR, &interface_addr)?;
    socket.set_multicast_if_v4(&interface_addr)?;

    let socket = Arc::new(UdpSocket::from(socket));

    let recv_buffer = vec![0; 4096];

    Ok((
        mDNSListener { recv: socket.clone(), recv_buffer },
        mDNSSender { service_name, send: socket },
    ))
}

const ADDR_ANY: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

#[cfg(not(target_os = "windows"))]
fn create_socket() -> io::Result<std::net::UdpSocket> {
    net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind((ADDR_ANY, MULTICAST_PORT))
}

#[cfg(target_os = "windows")]
fn create_socket() -> io::Result<std::net::UdpSocket> {
    net2::UdpBuilder::new_v4()?
        .reuse_address(true)?
        .bind((ADDR_ANY, MULTICAST_PORT))
}

/// An mDNS sender on a specific interface.
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct mDNSSender<> {
    service_name: String,
    send: Arc<UdpSocket>,
}

impl mDNSSender {
    /// Send multicasted DNS queries.
    pub async fn send_request(&mut self) -> Result<(), Error> {
        let mut builder = dns_parser::Builder::new_query(0, false);
        let prefer_unicast = false;
        builder.add_question(
            &self.service_name,
            prefer_unicast,
            dns_parser::QueryType::PTR,
            dns_parser::QueryClass::IN,
        );
        let packet_data = builder.build().unwrap();

        let addr = SocketAddr::new(MULTICAST_ADDR.into(), MULTICAST_PORT);

        self.send.send_to(&packet_data, addr).await?;
        Ok(())
    }
}

/// An mDNS listener on a specific interface.
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct mDNSListener {
    recv: Arc<UdpSocket>,
    recv_buffer: Vec<u8>,
}

impl mDNSListener {
    pub fn listen(mut self) -> impl Stream<Item = Result<Response, Error>> {
        try_stream! {
            loop {
                let (count, _) = self.recv.recv_from(&mut self.recv_buffer).await?;

                if count > 0 {
                    match dns_parser::Packet::parse(&self.recv_buffer[..count]) {
                        Ok(raw_packet) => yield Response::from_packet(&raw_packet),
                        Err(e) => log::warn!("{}, {:?}", e, &self.recv_buffer[..count]),
                    }
                }
            }
        }
    }
}
