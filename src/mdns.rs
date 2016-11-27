use {Error, Response};

use std::collections::VecDeque;

use mio::udp::*;
use dns;
use net2;

use net2::unix::UnixUdpBuilderExt;

/// The IP address for the mDNS multicast socket.
const MULTICAST_ADDR: &'static str = "224.0.0.251";

/// An mDNS discovery.
#[allow(non_camel_case_types)]
pub struct mDNS
{
    /// The name of the service that we are discovering.
    service_name: String,
    /// The UDP socket that we send/receive multicasts on.
    socket: UdpSocket,
    /// The DNS responses we have obtained so far.
    responses: VecDeque<Response>,
}

impl mDNS
{
    /// Creates a new mDNS discovery.
    pub fn new(service_name: &str) -> Result<Self, Error> {
        let multicast_addr = MULTICAST_ADDR.parse().unwrap();

        let socket = net2::UdpBuilder::new_v4()?
                                  .reuse_address(true)?
                                  .reuse_port(true)?
                                  .bind("0.0.0.0:5353")?;
        let socket = UdpSocket::from_socket(socket)?;

        let interface_addr = "192.168.1.100".parse().unwrap();

        socket.set_multicast_loop_v4(true)?;
        socket.set_multicast_ttl_v4(255)?;
        socket.join_multicast_v4(&multicast_addr, &interface_addr)?;

        Ok(mDNS {
            service_name: service_name.to_owned(),
            socket: socket,
            responses: VecDeque::new(),
        })
    }

    /// Send multicasted DNS queries.
    pub fn send(&mut self) -> Result<(), Error> {
        let mut builder = dns::Builder::new_query(0, false);
        builder.add_question(&self.service_name,
                             dns::QueryType::PTR,
                             dns::QueryClass::IN);
        let packet_data = builder.build().unwrap();

        let addr = "224.0.0.251:5353".parse().unwrap();
        self.socket.send_to(&packet_data, &addr)?;
        Ok(())
    }

    /// Attempts to receive data from the multicast socket.
    pub fn recv(&mut self) -> Result<(), Error> {
        let mut buffer: [u8; 10000] = [0; 10000];
        let (count, _) = self.socket.recv_from(&mut buffer)?.unwrap();
        let buffer = &buffer[0..count];

        if !buffer.is_empty() {
            let raw_packet = dns::Packet::parse(&buffer)?;
            self.responses.push_back(Response::from_packet(&raw_packet));
        }

        Ok(())
    }

    /// Consumes all DNS responses received so far.
    pub fn responses(&mut self) -> ::std::collections::vec_deque::Drain<Response> {
        self.responses.drain(..)
    }

    pub fn socket(&self) -> &UdpSocket { &self.socket }
}
