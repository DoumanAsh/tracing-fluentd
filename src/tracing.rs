use tracing_subscriber::registry::{LookupSpan, SpanRef};
use tracing_core::subscriber::Subscriber as Collect;
use tracing_subscriber::layer::Context;
use tracing_core::span::{Id, Attributes, Record};
use tracing_core::{Event, Field};

use crate::{Layer, FlattenFmt, NestedFmt, fluent, worker};

use core::fmt;

macro_rules! get_span {
    ($ctx:ident[$id:ident]) => {
        match $ctx.span($id) {
            Some(span) => span,
            None => return,
        }
    }
}

///Describes how compose event fields.
pub trait FieldFormatter: 'static {
    #[inline(always)]
    ///Handler for when `Layer::new_span` is invoked.
    ///
    ///By default uses span's extensions to store `fluent::Map` containing attributes of the span.
    fn new_span<C: Collect + for<'a> LookupSpan<'a>>(attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, C>) {
        let span = get_span!(ctx[id]);

        if span.extensions().get::<fluent::Map>().is_none() {
            let mut record = fluent::Map::new();
            attrs.record(&mut record);

            span.extensions_mut().insert(record);
        }
    }

    #[inline(always)]
    ///Handler for when `Layer::new_span` is invoked.
    ///
    ///By default uses span's extensions to store extra attributes of span within `fluent::Map`,
    ///created by  new_span, if any.
    fn on_record<C: Collect + for<'a> LookupSpan<'a>>(id: &Id, values: &Record<'_>, ctx: Context<'_, C>) {
        let span = get_span!(ctx[id]);

        let mut extensions = span.extensions_mut();
        if let Some(record) = extensions.get_mut::<fluent::Map>() {
            values.record(record);
        }
    }

    ///Handler for when `Layer::on_event` is invoked.
    ///
    ///Given `record` must be filled with data, after exiting this method, `record` is sent to the
    ///fluentd
    fn on_event<'a, R: LookupSpan<'a>>(record: &mut fluent::Record, event: &Event<'_>, current_span: Option<SpanRef<'a, R>>);
}

impl FieldFormatter for NestedFmt {
    #[inline(always)]
    fn on_event<'a, R: LookupSpan<'a>>(event_record: &mut fluent::Record, event: &Event<'_>, current_span: Option<SpanRef<'a, R>>) {
        use core::ops::DerefMut;

        event.record(event_record.deref_mut());
        let mut metadata = fluent::Map::new();

        if let Some(name) = event.metadata().file() {
            metadata.insert("file".to_owned(), name.to_owned().into());
        }
        if let Some(line) = event.metadata().line() {
            metadata.insert("line".to_owned(), line.into());
        }
        metadata.insert("module".to_owned(), event.metadata().target().to_owned().into());
        metadata.insert("level".to_owned(), event.metadata().level().to_owned().into());

        event_record.insert("metadata".to_owned(), metadata.into());

        if let Some(span) = current_span {
            let extensions = span.extensions();
            if let Some(record) = extensions.get::<fluent::Map>() {
                event_record.insert(span.name().to_owned(), record.clone().into());
            }
            while let Some(span) = span.parent() {
                if let Some(record) = extensions.get::<fluent::Map>() {
                    event_record.insert(span.name().to_owned(), record.clone().into());
                }
            }
        }
    }
}

impl FieldFormatter for FlattenFmt {
    #[inline(always)]
    fn on_event<'a, R: LookupSpan<'a>>(event_record: &mut fluent::Record, event: &Event<'_>, current_span: Option<SpanRef<'a, R>>) {
        use core::ops::DerefMut;

        event.record(event_record.deref_mut());
        if let Some(name) = event.metadata().file() {
            event_record.insert("file".to_owned(), name.to_owned().into());
        }
        if let Some(line) = event.metadata().line() {
            event_record.insert("line".to_owned(), line.into());
        }
        event_record.insert("module".to_owned(), event.metadata().target().to_owned().into());
        event_record.insert("level".to_owned(), event.metadata().level().to_owned().into());

        if let Some(span) = current_span {
            let extensions = span.extensions();
            if let Some(record) = extensions.get::<fluent::Map>() {
                event_record.update(record);
            }
            while let Some(span) = span.parent() {
                let extensions = span.extensions();
                if let Some(record) = extensions.get::<fluent::Map>() {
                    event_record.update(record)
                }
            }
        }
    }
}

impl tracing_core::field::Visit for fluent::Map {
    #[inline(always)]
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = format!("{:?}", value);
        self.insert(field.name().to_owned(), value.into());
    }

    #[inline(always)]
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field.name().to_owned(), value.into());
    }

    #[inline(always)]
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field.name().to_owned(), value.into());
    }

    #[inline(always)]
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field.name().to_owned(), value.into());
    }

    #[inline(always)]
    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field.name().to_owned(), value.to_owned().into());
    }

    #[inline(always)]
    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        let value = format!("{}", value);
        self.insert(field.name().to_owned(), value.into());
    }
}

impl<F: FieldFormatter, W: worker::Consumer, C: Collect + for<'a> LookupSpan<'a>> tracing_subscriber::layer::Layer<C> for Layer<F, W> {
    #[inline(always)]
    fn new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, C>) {
        F::new_span(attrs, id, ctx);
    }

    #[inline(always)]
    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, C>) {
        F::on_record(id, values, ctx);
    }

    #[inline(always)]
    fn on_enter(&self, _id: &Id, _ctx: Context<'_, C>) {
    }

    #[inline(always)]
    fn on_exit(&self, _id: &Id, _ctx: Context<'_, C>) {
    }

    #[inline(always)]
    fn on_close(&self, _id: Id, _ctx: Context<'_, C>) {
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
        let mut record = fluent::Record::now();

        F::on_event(&mut record, event, ctx.event_span(event));

        self.consumer.record(record);
    }
}

//TODO: Subscriber?
//impl<F: FieldFormatter, A: MakeWriter + 'static> tracing_core::subscriber::Subscriber for Builder<F, A> {
//    fn enabled(&self, _: &Metadata<'_>) -> bool {
//        true
//    }
//
//    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
//        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
//
//        hasher.update(attrs.metadata().name().as_bytes());
//        hasher.update(attrs.metadata().target().as_bytes());
//        if let Some(module) = attrs.metadata().module_path() {
//            hasher.update(module.as_bytes());
//        }
//        if let Some(file) = attrs.metadata().file() {
//            hasher.update(file.as_bytes());
//        }
//        if let Some(line) = attrs.metadata().line() {
//            hasher.update(&line.to_le_bytes());
//        }
//        for field in attrs.metadata().fields() {
//            hasher.update(field.name().as_bytes());
//        }
//
//        match core::num::NonZeroU64::new(hasher.digest()) {
//            Some(num) => Id::from_non_zero_u64(num),
//            None => Id::from_non_zero_u64(unsafe {
//                core::num::NonZeroU64::new_unchecked(u64::max_value())
//            }),
//        }
//    }
//
//    fn record(&self, _span: &Id, _values: &Record<'_>) {
//        todo!();
//    }
//
//    fn record_follows_from(&self, _span: &Id, _follows: &Id) {
//        todo!();
//    }
//
//    fn event(&self, _event: &Event<'_>) {
//        todo!();
//    }
//
//    fn enter(&self, _span: &Id) {
//        todo!();
//    }
//
//    fn exit(&self, _span: &Id) {
//        todo!();
//    }
//}
