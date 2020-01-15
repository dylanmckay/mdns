use crate::{Error, mDNS};

use std::time::Duration;
use mio;

pub struct Io
{
    pub poll: mio::Poll,
    pub events: mio::Events,
    token_accumulator: usize,
}

impl Io
{
    pub fn new() -> Result<Self, Error> {
        let poll = mio::Poll::new()?;
        let events = mio::Events::with_capacity(1024);

        Ok(Io {
            poll,
            events,
            token_accumulator: 0,
        })
    }

    pub fn poll(&mut self,
                mdns: &mut mDNS,
                timeout: Option<Duration>)
        -> Result<(), Error> {
        self.poll.poll(&mut self.events, timeout)?;

        for event in self.events.iter() {
            if event.readiness().is_readable() { mdns.recv(event.token())? };
            if event.readiness().is_writable() { mdns.send_if_ready(event.token())? };
        }

        Ok(())
    }

    pub fn create_token(&mut self) -> mio::Token {
        let token = mio::Token(self.token_accumulator);
        self.token_accumulator += 1;
        token
    }
}
