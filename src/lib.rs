extern crate mio;
extern crate dns_parser as dns;
extern crate net2;

use mio::*;
use mio::udp::*;

use net2::unix::UnixUdpBuilderExt;

const SERVER: Token = Token(0);

const MULTICAST_ADDR: &'static str = "224.0.0.251";

pub fn run() {
    let multicast_addr = MULTICAST_ADDR.parse().expect("failed to parse multicast addr");

    let tx = net2::UdpBuilder::new_v4().unwrap()
                              .reuse_address(true).expect("failed to set SO_REUSEADDR")
                              .reuse_port(true).expect("failed to set SO_REUSEPORT")
                              .bind("0.0.0.0:5353")
                              .expect("failed to bind");
    let tx = UdpSocket::from_socket(tx).expect("failed to wrap the socket");

    tx.set_multicast_loop_v4(true).unwrap();
    tx.set_multicast_ttl_v4(255).unwrap();
    tx.join_multicast_v4(&multicast_addr, &"192.168.1.100".parse().unwrap()).expect("failed to join multicast group");

    let poll = Poll::new().unwrap();
    // Start listening for incoming connections
    poll.register(&tx, SERVER, Ready::readable() | Ready::writable(),
                  PollOpt::edge()).unwrap();

    // Create storage for events
    let mut events = Events::with_capacity(1024);

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            println!("kind: {:?}", event.kind());
            match event.token() {
                SERVER => {
                    if event.kind().is_readable() {
                        let mut buffer: [u8; 10000] = [0; 10000];
                        let (count, _) = tx.recv_from(&mut buffer).expect("failed to read from the tx").unwrap();
                        let buffer = &buffer[0..count];

                        let packet = dns::Packet::parse(&buffer).expect("invalid DNS packet");

                        println!("name: {:#?}", packet);
                    }

                    if event.kind().is_writable() {
                        println!("sending a mDNS query");

                        let mut builder = dns::Builder::new_query(0, false);
                        builder.add_question("_googlecast._tcp.local",
                                             dns::QueryType::PTR,
                                             dns::QueryClass::IN);
                        let packet_data = builder.build().unwrap();

                        let addr = "224.0.0.251:5353".parse().unwrap();
                        tx.send_to(&packet_data, &addr).unwrap().unwrap();
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
