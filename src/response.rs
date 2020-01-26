use std::net;
use std::net::{IpAddr, SocketAddr};

/// A DNS response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub answers: Vec<Record>,
    pub nameservers: Vec<Record>,
    pub additional: Vec<Record>,
}

/// Any type of DNS record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub name: String,
    pub class: dns_parser::Class,
    pub ttl: u32,
    pub kind: RecordKind,
}

/// A specific DNS record variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordKind {
    A(net::Ipv4Addr),
    AAAA(net::Ipv6Addr),
    CNAME(String),
    MX {
        preference: u16,
        exchange: String,
    },
    NS(String),
    SRV {
        priority: u16,
        weight: u16,
        port: u16,
        target: String,
    },
    TXT(Vec<String>),
    PTR(String),
    /// A record kind that hasn't been implemented by this library yet.
    Unimplemented(Vec<u8>),
}

impl Response {
    pub fn from_packet(packet: &dns_parser::Packet) -> Self {
        Response {
            answers: packet
                .answers
                .iter()
                .map(Record::from_resource_record)
                .collect(),
            nameservers: packet
                .nameservers
                .iter()
                .map(Record::from_resource_record)
                .collect(),
            additional: packet
                .additional
                .iter()
                .map(Record::from_resource_record)
                .collect(),
        }
    }

    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.answers
            .iter()
            .chain(self.nameservers.iter())
            .chain(self.additional.iter())
    }

    pub fn is_empty(&self) -> bool {
        self.answers.is_empty() && self.nameservers.is_empty() && self.additional.is_empty()
    }

    pub fn ip_addr(&self) -> Option<IpAddr> {
        self.records().find_map(|record| match record.kind {
            RecordKind::A(addr) => Some(addr.into()),
            RecordKind::AAAA(addr) => Some(addr.into()),
            _ => None,
        })
    }

    pub fn hostname(&self) -> Option<&str> {
        self.records().find_map(|record| match record.kind {
            RecordKind::PTR(ref host) => Some(host.as_str()),
            _ => None,
        })
    }

    pub fn port(&self) -> Option<u16> {
        self.records().find_map(|record| match record.kind {
            RecordKind::SRV { port, .. } => Some(port),
            _ => None,
        })
    }

    pub fn socket_address(&self) -> Option<SocketAddr> {
        Some((self.ip_addr()?, self.port()?).into())
    }

    pub fn txt_records(&self) -> impl Iterator<Item = &str> {
        self.records()
            .filter_map(|record| match record.kind {
                RecordKind::TXT(ref txt) => Some(txt),
                _ => None,
            })
            .flat_map(|txt| txt.iter())
            .map(|txt| txt.as_str())
    }
}

impl Record {
    fn from_resource_record(rr: &dns_parser::ResourceRecord) -> Self {
        Record {
            name: rr.name.to_string(),
            class: rr.cls,
            ttl: rr.ttl,
            kind: RecordKind::from_rr_data(&rr.data),
        }
    }
}

impl RecordKind {
    fn from_rr_data(data: &dns_parser::RData) -> Self {
        use dns_parser::RData;

        match *data {
            RData::A(dns_parser::rdata::a::Record(addr)) => RecordKind::A(addr),
            RData::AAAA(dns_parser::rdata::aaaa::Record(addr)) => RecordKind::AAAA(addr),
            RData::CNAME(ref name) => RecordKind::CNAME(name.to_string()),
            RData::MX(dns_parser::rdata::mx::Record {
                preference,
                ref exchange,
            }) => RecordKind::MX {
                preference,
                exchange: exchange.to_string(),
            },
            RData::NS(ref name) => RecordKind::NS(name.to_string()),
            RData::PTR(ref name) => RecordKind::PTR(name.to_string()),
            RData::SRV(dns_parser::rdata::srv::Record {
                priority,
                weight,
                port,
                ref target,
            }) => RecordKind::SRV {
                priority,
                weight,
                port,
                target: target.to_string(),
            },
            RData::TXT(ref txt) => RecordKind::TXT(
                txt.iter()
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                    .collect(),
            ),
            RData::SOA(..) => {
                RecordKind::Unimplemented("SOA record handling is not implemented".into())
            }
            RData::Unknown(data) => RecordKind::Unimplemented(data.to_owned()),
        }
    }
}
