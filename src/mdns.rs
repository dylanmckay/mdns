use {Error, Response, Io};

use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::{io, time};

use mio::net::UdpSocket;
use mio;
use dns;
use net2;
use get_if_addrs;

#[cfg(not(target_os = "windows"))]
use net2::unix::UnixUdpBuilderExt;

/// The IP address for the mDNS multicast socket.
const MULTICAST_ADDR: &'static str = "224.0.0.251";
const MULTICAST_PORT: u16 = 5353;

/// The minimum amount of time between outgoing DNS requests.
const MIN_MILLIS_BETWEEN_DNS_REQUESTS: u64 = 1_000;

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
    /// The request throttler.
    request_throttle: Throttle,
}

/// An mDNS discovery on a specific interface.
struct InterfaceDiscovery
{
    token: mio::Token,
    socket: UdpSocket,
}

/// A request throttler, so that we do not saturate the network.
#[derive(Clone, Debug)]
enum Throttle {
    /// No requests have been sent by us yet.
    NothingSent {
        /// The minimum interval of time between requests.
        minimum_interval: time::Duration,
    },
    /// Requests are being sent and we are keeping track of timestamps to
    /// limit the number of requests.
    Running {
        /// The minimum interval of time between requests.
        minimum_interval: time::Duration,
        /// When the last DNS request was sent by us.
        last_request_at: time::Instant,
    },
}

impl mDNS
{
    pub fn new(service_name: &str, io: &mut Io) -> Result<Self, Error> {
        let interfaces: Result<Vec<_>, _> = get_if_addrs::get_if_addrs()
            .unwrap()
            .into_iter()
            .filter_map(|addr| {
                if let IpAddr::V4(socket_addr) = addr.ip() {
                    Some(InterfaceDiscovery::new(&socket_addr, io))
            } else {
                None
            }
        }).collect();

        let interfaces = interfaces?;

        Ok(mDNS {
            service_name: service_name.to_owned(),
            sockets: interfaces,
            responses: VecDeque::new(),
            request_throttle: Throttle::new(time::Duration::from_millis(MIN_MILLIS_BETWEEN_DNS_REQUESTS)),
        })
    }

    pub fn recv(&mut self, token: mio::Token) -> Result<(), Error> {
        let interface = self.sockets.iter_mut().find(|sock| sock.token == token).unwrap();
        self.responses.extend(interface.recv()?);
        Ok(())
    }

    pub fn send_if_ready(&mut self, token: mio::Token) -> Result<(), Error> {
        if self.request_throttle.is_open() {
            let interface = self.sockets.iter_mut().find(|sock| sock.token == token).unwrap();
            interface.send(&self.service_name)?;

            self.request_throttle.mark();
        }

        Ok(())
    }

    /// Gets the mio tokens of all clients.
    pub fn client_tokens(&self) -> Vec<mio::Token> {
        self.sockets.iter().map(|s| s.token).collect()
    }

    /// Consumes all DNS responses received so far.
    pub fn responses(&mut self) -> ::std::collections::vec_deque::Drain<Response> {
        self.responses.drain(..)
    }
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
    fn new(interface_addr: &Ipv4Addr, io: &mut Io) -> Result<Self, Error> {
        let multicast_addr = MULTICAST_ADDR.parse().unwrap();

        let socket = InterfaceDiscovery::create_socket()?;
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

impl Throttle {
    /// Creates a new request throttler.
    pub fn new(minimum_interval: time::Duration) -> Self {
        Throttle::NothingSent { minimum_interval }
    }

    /// Checks if the throttle is open for more requests.
    pub fn is_open(&self) -> bool {
        match *self {
            Throttle::NothingSent { .. }  => true,
            Throttle::Running { minimum_interval, last_request_at } => last_request_at.elapsed() >= minimum_interval,
        }
    }

    /// Marks that another request has been sent and adjusts limits accordingly.
    pub fn mark(&mut self) {
        *self = Throttle::Running {
            minimum_interval: self.minimum_interval(),
            last_request_at: time::Instant::now(),
        };
    }

    /// Gets the minimum interval between requests.
    pub fn minimum_interval(&self) -> time::Duration {
        match *self {
            Throttle::NothingSent { minimum_interval } => minimum_interval,
            Throttle::Running { minimum_interval, .. } => minimum_interval,
        }
    }
}

