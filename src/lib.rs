//! [tracing](https://github.com/tokio-rs/tracing) for [fluentd](https://www.fluentd.org/).
//!
//!## Example
//!
//!```rust
//!use tracing_subscriber::layer::SubscriberExt;
//!
//!let layer = tracing_fluentd::Builder::new("rust").flatten().layer().expect("Create layer");
//!let sub = tracing_subscriber::Registry::default().with(layer);
//!let guard = tracing::subscriber::set_default(sub);
//!```

#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

use std::net::{TcpStream, SocketAddrV4, SocketAddr, Ipv4Addr};
use std::io::Write;
use core::marker::PhantomData;

mod tracing;
pub mod fluent;
mod worker;

pub use self::tracing::FieldFormatter;

///Policy to insert span data as object.
///
///Specifically, any span's or event metadata's attributes are associated with its name inside
///record.
///For example having span `lolka` would add key `lolka` to the record, with span's attributes as
///value.
///
///Special case is event metadata which is always inserted with key `metadata` and contains
///information such location in code and event level.
pub struct NestedFmt;
///Policy to insert span data as flattent object.
///
///Specifically, any span's or event metadata's attributes are inserted at the root of event
///record.
///For example, having span `lolka` with attribute `arg: 1` would result in `arg: 1` to be inserted
///alongside `message` and other attributes of the event.
pub struct FlattenFmt;

///Describers creation of sink for `tracing` record.
pub trait MakeWriter: 'static + Send {
    ///Writer type
    type Writer: Write;

    ///Creates instance of `Writer`.
    ///
    ///It should be noted that it is ok to cache `Writer`.
    ///
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
    use core::time::Duration;

    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 24224));
    TcpStream::connect_timeout(&addr, Duration::from_secs(1))
}

///`tracing`'s Layer
pub struct Layer<F, C> {
    consumer: C,
    _fmt: PhantomData<F>,
}

///Builder to enable forwarding `tracing` events towards the `fluentd` server.
///
///## Type params
///
///- `F` - Attributes formatter, determines how to compose `fluent::Record`.
///- `A` - function that returns `Fluentd` wrter. Default is to create tcp socket towards `127.0.0.1:24224` with timeout of 1s.
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
    ///Configures to flatten span/metadata attributes within record.
    ///Instead of the default nesting behavior.
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
    ///Normally fluentd server expects connection to be closed immediately upon sending records.
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
