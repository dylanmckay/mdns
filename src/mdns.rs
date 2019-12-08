use crate::{Error, Response};

use std::{io, net::Ipv4Addr};

use async_stream::try_stream;
use futures_core::Stream;
use futures_util::future::join_all;
use futures_util::stream::select_all;
use net2;
use tokio::net::{
    udp::{RecvHalf, SendHalf},
    UdpSocket,
};

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
    let (listener, sender) = new_interface(interface_addr)?;
    let listeners = vec![listener];
    let senders = vec![sender];

    Ok((
        mDNSListener { sockets: listeners },
        mDNSSender {
            service_name,
            sockets: senders,
        },
    ))
}

/// An mDNS discovery.
#[allow(non_camel_case_types)]
pub struct mDNSListener {
    sockets: Vec<InterfaceListener>,
}

impl mDNSListener {
    pub fn listen(self) -> impl Stream<Item = Result<Response, Error>> {
        select_all(
            self.sockets
                .into_iter()
                .map(|socket| Box::pin(socket.listen())),
        )
    }
}

#[allow(non_camel_case_types)]
pub struct mDNSSender {
    service_name: String,
    sockets: Vec<InterfaceSender>,
}

impl mDNSSender {
    /// Send out a mDNS request using all interfaces.
    pub async fn send_request(&mut self) {
        let name = &self.service_name;
        let sockets = &mut self.sockets;
        join_all(sockets.iter_mut().map(|socket| socket.send_request(name))).await;
    }
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

/// Creates a new mDNS discovery.
fn new_interface(interface_addr: Ipv4Addr) -> Result<(InterfaceListener, InterfaceSender), Error> {
    let socket = create_socket()?;
    let socket = UdpSocket::from_std(socket)?;

    socket.set_multicast_loop_v4(false)?;
    socket.join_multicast_v4(MULTICAST_ADDR, interface_addr)?;

    let (recv, send) = socket.split();

    let recv_buffer = vec![0; 4096];

    Ok((
        InterfaceListener { recv, recv_buffer },
        InterfaceSender { send },
    ))
}

/// An mDNS sender on a specific interface.
struct InterfaceSender {
    send: SendHalf,
}

impl InterfaceSender {
    /// Send multicasted DNS queries.
    async fn send_request(&mut self, service_name: &str) -> Result<(), Error> {
        let mut builder = dns_parser::Builder::new_query(0, false);
        let prefer_unicast = false;
        builder.add_question(
            service_name,
            prefer_unicast,
            dns_parser::QueryType::PTR,
            dns_parser::QueryClass::IN,
        );
        let packet_data = builder.build().unwrap();

        let addr = SocketAddr::new(MULTICAST_ADDR.into(), MULTICAST_PORT);

        self.send.send_to(&packet_data, &addr).await?;
        Ok(())
    }
}

/// An mDNS listener on a specific interface.
struct InterfaceListener {
    recv: RecvHalf,
    recv_buffer: Vec<u8>,
}

impl InterfaceListener {
    pub fn listen(mut self) -> impl Stream<Item = Result<Response, Error>> {
        try_stream! {
            loop {
                let (count, _) = self.recv.recv_from(&mut self.recv_buffer).await?;

                if count > 0 {
                    match dns_parser::Packet::parse(&self.recv_buffer[..count]) {
                        Ok(raw_packet) => yield Response::from_packet(&raw_packet),
                        Err(e) => eprintln!("{}, {:?}", e, &self.recv_buffer[..count])
                    }
                }
            }
        }
    }
}
