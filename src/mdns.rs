use {Error, Response, Io};

use std::collections::VecDeque;
use std::net::{SocketAddr, Ipv4Addr};
use std::net::ToSocketAddrs;

use mio::net::UdpSocket;
use mio;
use dns;
use net2;
use ifaces;

use net2::unix::UnixUdpBuilderExt;

/// The IP address for the mDNS multicast socket.
const MULTICAST_ADDR: &'static str = "224.0.0.251";
const MULTICAST_PORT: u16 = 5353;

/// An mDNS discovery.
#[allow(non_camel_case_types)]
pub struct mDNS
{
    /// The name of the service that we are discovering.
    service_name: String,
    /// The UDP sockets that we send/receive multicasts on.
    /// There will be one per socket.
    sockets: Vec<InterfaceDiscovery>,
    /// The DNS responses we have obtained so far.
    responses: VecDeque<Response>,
}

/// An mDNS discovery on a specific interface.
struct InterfaceDiscovery
{
    token: mio::Token,
    socket: UdpSocket,
}

impl mDNS
{
    pub fn new(service_name: &str, io: &mut Io) -> Result<Self, Error> {
        let interfaces: Result<Vec<_>, _> = ifaces::Interface::get_all().unwrap().into_iter().filter_map(|iface| {
            if let Some(SocketAddr::V4(socket_addr)) = iface.addr {
                Some(InterfaceDiscovery::new(socket_addr.ip(), io))
            } else {
                None
            }
        }).collect();

        let interfaces = interfaces?;

        Ok(mDNS {
            service_name: service_name.to_owned(),
            sockets: interfaces,
            responses: VecDeque::new(),
        })
    }

    pub fn recv(&mut self, token: mio::Token) -> Result<(), Error> {
        let interface = self.sockets.iter_mut().find(|sock| sock.token == token).unwrap();
        self.responses.extend(interface.recv()?);
        Ok(())
    }

    pub fn send(&mut self, token: mio::Token) -> Result<(), Error> {
        let interface = self.sockets.iter_mut().find(|sock| sock.token == token).unwrap();
        interface.send(&self.service_name)
    }

    /// Consumes all DNS responses received so far.
    pub fn responses(&mut self) -> ::std::collections::vec_deque::Drain<Response> {
        self.responses.drain(..)
    }
}

impl InterfaceDiscovery
{
    /// Creates a new mDNS discovery.
    fn new(interface_addr: &Ipv4Addr, io: &mut Io) -> Result<Self, Error> {
        let multicast_addr = MULTICAST_ADDR.parse().unwrap();

        let socket = net2::UdpBuilder::new_v4()?
                                  .reuse_address(true)?
                                  .reuse_port(true)?
                                  .bind(("0.0.0.0", MULTICAST_PORT))?;
        let socket = UdpSocket::from_socket(socket)?;

        socket.set_multicast_loop_v4(true)?;
        socket.set_multicast_ttl_v4(255)?;
        socket.join_multicast_v4(&multicast_addr, interface_addr)?;

        let token = io.create_token();
        io.poll.register(&socket,
                         token,
                         mio::Ready::readable() | mio::Ready::writable(),
                         mio::PollOpt::edge())?;

        Ok(InterfaceDiscovery {
            token,
            socket,
        })
    }

    /// Send multicasted DNS queries.
    fn send(&mut self, service_name: &str) -> Result<(), Error> {
        let mut builder = dns::Builder::new_query(0, false);
        let prefer_unicast = false;
        builder.add_question(service_name,
                             prefer_unicast,
                             dns::QueryType::PTR,
                             dns::QueryClass::IN);
        let packet_data = builder.build().unwrap();

        let addr = (MULTICAST_ADDR, MULTICAST_PORT).to_socket_addrs().unwrap().next().unwrap();
        self.socket.send_to(&packet_data, &addr)?;
        Ok(())
    }

    /// Attempts to receive data from the multicast socket.
    fn recv(&mut self) -> Result<Vec<Response>, Error> {
        let mut buffer: [u8; 10000] = [0; 10000];
        let (count, _) = self.socket.recv_from(&mut buffer)?;
        let buffer = &buffer[0..count];

        if !buffer.is_empty() {
            let raw_packet = dns::Packet::parse(&buffer)?;
            return Ok(vec![Response::from_packet(&raw_packet)]);
        }

        Ok(vec![])
    }
}
