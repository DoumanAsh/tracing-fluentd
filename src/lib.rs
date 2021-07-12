//! [tracing](https://github.com/tokio-rs/tracing) subscriber for [fluentd](https://www.fluentd.org/).
//!

#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

use std::net::{TcpStream, SocketAddrV4, SocketAddr, Ipv4Addr};
use std::io::Write;

mod tracing;
mod fluent;

///Describers creation of sink for `tracing` record.
pub trait MakeWriter {
    ///Writer type
    type Writer: Write;

    ///Creates instance of `Writer`.
    ///
    ///It should be noted that it is ok to cache `Writer`
    ///In case of failure working with writer, subscriber shall retry at least once
    fn make(&self) -> std::io::Result<Self::Writer>;
}

impl<W: Write, T: Fn() -> std::io::Result<W>> MakeWriter for T {
    type Writer = W;
    #[inline(always)]
    fn make(&self) -> std::io::Result<Self::Writer> {
        (self)()
    }
}

fn default() -> std::io::Result<TcpStream> {
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 24224));
    TcpStream::connect(addr)
}

///Subscriber for Fluentd forward endpoint.
///
///## Type params
///
///`A` - function that returns `Fluentd` addr. Default value is to return `127.0.0.1:24224`
pub struct Subscriber<A=fn() -> std::io::Result<TcpStream>> {
    tag: &'static str,
    writer: A,
}

impl Subscriber {
    #[inline(always)]
    ///Creates default configuration.
    ///
    ///## Params:
    ///
    ///`tag` - Event category to send for each record.
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            writer: default,
        }
    }
}

impl<A: MakeWriter> Subscriber<A> {
    ///Provides callback to get `fluentd` server address.
    pub fn with_addr<MW: MakeWriter>(self, writer: MW) -> Subscriber<MW> {
        Subscriber {
            tag: self.tag,
            writer
        }
    }
}
