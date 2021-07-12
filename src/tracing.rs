use tracing_subscriber::registry::LookupSpan;
use tracing_core::subscriber::Subscriber as Collect;
use tracing_subscriber::layer::Context;
use tracing_core::span::{Id, Attributes, Record};
use tracing_core::{Event, Metadata};

use super::{Subscriber, MakeWriter};

macro_rules! get_span {
    ($ctx:ident[$id:ident]) => {
        match $ctx.span($id) {
            Some(span) => span,
            None => return,
        }
    }
}

impl<A: MakeWriter + 'static, C: Collect + for<'a> LookupSpan<'a>> tracing_subscriber::layer::Layer<C> for Subscriber<A> {
    fn new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, C>) {
        let span = get_span!(ctx[id]);

        let mut extensions = span.extensions_mut();
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, C>) {
        let span = get_span!(ctx[id]);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, C>) {
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, C>) {
    }

    fn on_close(&self, id: Id, ctx: Context<'_, C>) {
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
    }
}

impl<A: MakeWriter + 'static> tracing_core::subscriber::Subscriber for Subscriber<A> {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let mut hasher = xxhash_rust::xxh3::Xxh3::new();

        hasher.update(attrs.metadata().name().as_bytes());
        hasher.update(attrs.metadata().target().as_bytes());
        if let Some(module) = attrs.metadata().module_path() {
            hasher.update(module.as_bytes());
        }
        if let Some(file) = attrs.metadata().file() {
            hasher.update(file.as_bytes());
        }
        if let Some(line) = attrs.metadata().line() {
            hasher.update(&line.to_le_bytes());
        }
        for field in attrs.metadata().fields() {
            hasher.update(field.name().as_bytes());
        }

        match core::num::NonZeroU64::new(hasher.digest()) {
            Some(num) => Id::from_non_zero_u64(num),
            None => Id::from_non_zero_u64(unsafe {
                core::num::NonZeroU64::new_unchecked(u64::max_value())
            }),
        }
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        todo!();
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        todo!();
    }

    fn event(&self, event: &Event<'_>) {
        todo!();
    }

    fn enter(&self, span: &Id) {
        todo!();
    }

    fn exit(&self, span: &Id) {
        todo!();
    }
}
