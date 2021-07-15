//! [tracing](https://github.com/tokio-rs/tracing) subscriber for [fluentd](https://www.fluentd.org/).
//!

#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

use std::net::{TcpStream, SocketAddrV4, SocketAddr, Ipv4Addr};
use std::io::Write;
use core::marker::PhantomData;

mod tracing;
pub mod fluent;
mod worker;

///Describers how to format event data
pub trait FieldFormatter: tracing::FieldFormatter {
}

///Policy to insert span data as object.
pub struct NestedFmt;
///Policy to insert span data as flattent object.
pub struct FlattenFmt;

impl FieldFormatter for NestedFmt {
}

impl FieldFormatter for FlattenFmt {
}

///Describers creation of sink for `tracing` record.
pub trait MakeWriter: 'static + Send {
    ///Writer type
    type Writer: Write;

    ///Creates instance of `Writer`.
    ///
    ///It should be noted that it is ok to cache `Writer`
    ///In case of failure working with writer, subscriber shall retry at least once
    fn make(&self) -> std::io::Result<Self::Writer>;
}

impl<W: Write, T: 'static + Send + Fn() -> std::io::Result<W>> MakeWriter for T {
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

///`tracing`'s Layer
pub struct Layer<F, C> {
    consumer: C,
    _fmt: PhantomData<F>,
}

///Builder for Fluentd forward endpoint.
///
///## Type params
///
///`A` - function that returns `Fluentd` addr. Default value is to return `127.0.0.1:24224`
pub struct Builder<F=NestedFmt, A=fn() -> std::io::Result<TcpStream>> {
    tag: &'static str,
    writer: A,
    _fmt: PhantomData<F>
}

impl Builder {
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
            _fmt: PhantomData,
        }
    }
}

impl<A: MakeWriter> Builder<NestedFmt, A> {
    #[inline(always)]
    ///Provides callback to get `fluentd` server address.
    pub fn flatten(self) -> Builder<FlattenFmt, A> {
        Builder {
            tag: self.tag,
            writer: self.writer,
            _fmt: PhantomData,
        }
    }
}

impl<F: FieldFormatter, A: MakeWriter> Builder<F, A> {
    #[inline(always)]
    ///Provides callback to get writer where to write records.
    ///
    ///Normally fluentd server requires to abort connection immediately
    ///hence created writer is dropped immediately upon writing being finished.
    pub fn with_writer<MW: MakeWriter>(self, writer: MW) -> Builder<F, MW> {
        Builder {
            tag: self.tag,
            writer,
            _fmt: PhantomData,
        }
    }

    #[inline(always)]
    ///Creates `tracing` layer.
    ///
    ///`Error` can happen during creation of worker thread.
    pub fn layer(self) -> Result<Layer<F, worker::ThreadWorker>, std::io::Error> {
        let consumer = worker::thread(self.tag, self.writer)?;

        Ok(Layer {
            consumer,
            _fmt: PhantomData,
        })
    }
}
