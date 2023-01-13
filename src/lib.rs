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
use core::num;

mod tracing;
pub mod fluent;
mod worker;
mod default_writers;

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
    fmt: F,
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
    fmt: F,
    max_msg_record: usize,
}

impl Builder {
    #[inline(always)]
    ///Creates default configuration.
    ///
    ///## Params:
    ///
    ///`tag` - Event category to send for each record.
    pub fn new(tag: &'static str) -> Self {
        const DEFAULT_MAX_MSG_RECORD: usize = 10;
        Self {
            tag,
            writer: default,
            fmt: NestedFmt,
            max_msg_record: DEFAULT_MAX_MSG_RECORD,
        }
    }

    #[inline(always)]
    ///Provides max message record to fetch up.
    pub fn with_max_msg_record(self, max_msg_record: num::NonZeroUsize) -> Self {
        Self {
            tag: self.tag,
            writer: self.writer,
            fmt: self.fmt,
            max_msg_record: max_msg_record.get()
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
            fmt: FlattenFmt,
            max_msg_record: self.max_msg_record,
        }
    }
}

impl<F: FieldFormatter, A: MakeWriter> Builder<F, A> {
    #[inline(always)]
    ///Provides formatter.
    pub fn with_formatter<NF: FieldFormatter>(self, fmt: NF) -> Builder<NF, A> {
        Builder {
            tag: self.tag,
            writer: self.writer,
            fmt,
            max_msg_record: self.max_msg_record,
        }
    }

    #[inline(always)]
    ///Provides callback to get writer where to write records.
    ///
    ///Normally fluentd server expects connection to be closed immediately upon sending records.
    ///hence created writer is dropped immediately upon writing being finished.
    pub fn with_writer<MW: MakeWriter>(self, writer: MW) -> Builder<F, MW> {
        Builder {
            tag: self.tag,
            writer,
            fmt: self.fmt,
            max_msg_record: self.max_msg_record,
        }
    }

    #[inline(always)]
    ///Creates `tracing` layer.
    ///
    ///If you do not want to create multiple threads, consider using
    ///`layer_guarded`/`layer_from_guard`.
    ///
    ///`Error` can happen during creation of worker thread.
    pub fn layer(self) -> Result<Layer<F, worker::ThreadWorker>, std::io::Error> {
        let consumer = worker::thread(self.tag, self.writer, self.max_msg_record)?;

        Ok(Layer {
            consumer,
            fmt: self.fmt,
        })
    }

    #[inline]
    ///Creates `tracing` layer, returning guard that allows to stop `fluentd` worker on `Drop`.
    ///
    ///This may be necessary due to bad API that `tracing` provides to control lifetime of global
    ///logger. As underlying implementations employs caching, it needs to perform flush once logger
    ///is no longer necessary hence this API is provided.
    ///
    ///`Error` can happen during creation of worker thread.
    pub fn layer_guarded(self) -> Result<(Layer<F, worker::WorkerChannel>, FlushingGuard), std::io::Error> {
        let consumer = worker::thread(self.tag, self.writer, self.max_msg_record)?;
        let guard = FlushingGuard(consumer);
        let layer = Layer {
            consumer: worker::WorkerChannel(guard.0.sender()),
            fmt: self.fmt,
        };

        Ok((layer, guard))
    }

    #[inline(always)]
    ///Creates `tracing` layer, using guard returned by  `layer_guarded`.
    ///
    ///Specifically, it will use the same worker thread as first instance of `layer_guarded`,
    ///without affecting lifetime of `guard`.
    ///Hence once `guard` is dropped, worker for all connected layers will stop sending logs.
    ///
    ///`Error` can happen during creation of worker thread.
    pub fn layer_from_guard(self, guard: &FlushingGuard) -> Layer<F, worker::WorkerChannel> {
        Layer {
            consumer: worker::WorkerChannel(guard.0.sender()),
            fmt: self.fmt,
        }
    }
}

#[repr(transparent)]
///Guard that flushes and terminates `fluentd` worker.
///
///Droping this guard should be done only when `Layer` is no longer needed.
///
///As part of destructor, it awaits to finish flushing `fluentd` records.
pub struct FlushingGuard(worker::ThreadWorker);

impl Drop for FlushingGuard {
    fn drop(&mut self) {
        self.0.stop();
    }
}
