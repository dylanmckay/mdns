use {Error, Response};

use std::{
    io,
    net::{IpAddr, Ipv4Addr, ToSocketAddrs},
};

use dns;
use get_if_addrs;
use net2;
use tokio_udp::UdpSocket;

use futures::{
    try_ready,
    Async::{NotReady, Ready},
    Poll, Stream,
};

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

/// The IP address for the mDNS multicast socket.
const MULTICAST_ADDR: &'static str = "224.0.0.251";
const MULTICAST_PORT: u16 = 5353;

/// An mDNS discovery.
#[allow(non_camel_case_types)]
pub struct mDNS {
    /// The name of the service that we are discovering.
    service_name: String,
    /// The UDP sockets that we send/receive multicasts on.
    /// There will be one per socket.
    sockets: Vec<InterfaceDiscovery>,
    next_to_poll: usize,
}

impl mDNS {
    pub fn new(service_name: &str) -> Result<Self, Error> {
        let interfaces: Result<Vec<_>, _> = get_if_addrs::get_if_addrs()
            .unwrap()
            .into_iter()
            .filter_map(|addr| {
                if let IpAddr::V4(socket_addr) = addr.ip() {
                    Some(InterfaceDiscovery::new(&socket_addr))
                } else {
                    None
                }
            })
            .collect();

        let interfaces = interfaces?;

        Ok(mDNS {
            service_name: service_name.to_owned(),
            sockets: interfaces,
            next_to_poll: 0,
        })
    }

    /// Send out a mDNS request using all interfaces.
    pub fn send_request(&mut self) -> Result<(), Error> {
        let service_name = &self.service_name;
        self.sockets
            .iter_mut()
            .try_for_each(|i| i.send_request(&service_name))
    }
}

impl Stream for mDNS {
    type Item = Response;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut next_to_poll = self.next_to_poll;
        let mut rounds = 0;

        loop {
            if let Ready(res) = self.sockets[next_to_poll].poll()? {
                self.next_to_poll = (next_to_poll + 1) % self.sockets.len();
                return Ok(Ready(res));
            }

            rounds += 1;
            if rounds == self.sockets.len() {
                return Ok(NotReady);
            }

            next_to_poll = (next_to_poll + 1) % self.sockets.len();
        }
    }
}

/// An mDNS discovery on a specific interface.
struct InterfaceDiscovery {
    socket: UdpSocket,
    recv_buffer: Vec<u8>,
}

impl InterfaceDiscovery {
    #[cfg(not(target_os = "windows"))]
    fn create_socket() -> io::Result<std::net::UdpSocket> {
        net2::UdpBuilder::new_v4()?
            .reuse_address(true)?
            .reuse_port(true)?
            .bind(("0.0.0.0", MULTICAST_PORT))
    }

    #[cfg(target_os = "windows")]
    fn create_socket() -> io::Result<std::net::UdpSocket> {
        net2::UdpBuilder::new_v4()?
            .reuse_address(true)?
            .bind(("0.0.0.0", MULTICAST_PORT))
    }

    /// Creates a new mDNS discovery.
    fn new(interface_addr: &Ipv4Addr) -> Result<Self, Error> {
        let multicast_addr = MULTICAST_ADDR.parse().unwrap();

        let socket = Self::create_socket()?;
        let socket = UdpSocket::from_std(socket, &Default::default())?;

        socket.set_multicast_loop_v4(true)?;
        socket.set_multicast_ttl_v4(255)?;
        socket.join_multicast_v4(&multicast_addr, interface_addr)?;

        let recv_buffer = vec![0; 4096];

        Ok(InterfaceDiscovery {
            socket,
            recv_buffer,
        })
    }

    /// Send multicasted DNS queries.
    fn send_request(&mut self, service_name: &str) -> Result<(), Error> {
        let mut builder = dns::Builder::new_query(0, false);
        let prefer_unicast = false;
        builder.add_question(
            service_name,
            prefer_unicast,
            dns::QueryType::PTR,
            dns::QueryClass::IN,
        );
        let packet_data = builder.build().unwrap();

        let addr = (MULTICAST_ADDR, MULTICAST_PORT)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        self.socket
            .poll_send_to(&packet_data, &addr)
            .map_err(Into::into)
            .map(|_| ())
    }
}

impl Stream for InterfaceDiscovery {
    type Item = Response;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let (count, _) = try_ready!(self.socket.poll_recv_from(&mut self.recv_buffer));

            if count > 0 {
                let raw_packet = dns::Packet::parse(&self.recv_buffer[..count])?;
                return Ok(Ready(Some(Response::from_packet(&raw_packet))));
            }
        }
    }
}
