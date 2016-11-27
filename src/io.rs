use {Error, mDNS};

use std::time::Duration;
use mio;

pub struct Io
{
    pub token: mio::Token,
    pub poll: mio::Poll,
    pub events: mio::Events,
}

impl Io
{
    pub fn new() -> Result<Self, Error> {
        let poll = mio::Poll::new()?;
        let events = mio::Events::with_capacity(1024);

        Ok(Io {
            token: mio::Token(0),
            poll: poll,
            events: events,
        })
    }

    pub fn register(&mut self,
                    mdns: &mDNS) -> Result<(), Error> {
        self.poll.register(mdns.socket(),
                           self.token,
                           mio::Ready::readable() | mio::Ready::writable(),
                           mio::PollOpt::edge())?;
        Ok(())
    }

    pub fn poll(&mut self,
                mdns: &mut mDNS,
                timeout: Option<Duration>)
        -> Result<(), Error> {
        self.poll.poll(&mut self.events, timeout)?;

        for event in self.events.iter() {
            assert_eq!(event.token(), self.token);

            if event.kind().is_readable() { mdns.recv()? };
            if event.kind().is_writable() { mdns.send()? };
        }

        Ok(())
    }
}
