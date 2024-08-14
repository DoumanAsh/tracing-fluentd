# tracing-fluentd

[![Crates.io](https://img.shields.io/crates/v/tracing-fluentd.svg)](https://crates.io/crates/tracing-fluentd)
[![Documentation](https://docs.rs/tracing-fluentd/badge.svg)](https://docs.rs/crate/tracing-fluentd/)
[![Build](https://github.com/DoumanAsh/tracing-fluentd/workflows/Rust/badge.svg)](https://github.com/DoumanAsh/tracing-fluentd/actions?query=workflow%3ARust)

Enables forwarding of `tracing` events towards the `fluentd` server.

## Features

- `event_time` - Specifies to encode timestamp as EventTime instead of default unix timestamp

## Example

```rust
use tracing_subscriber::layer::SubscriberExt;

let layer = tracing_fluentd::Builder::new("rust").flatten().layer().expect("Create layer");
let sub = tracing_subscriber::Registry::default().with(layer);
let guard = tracing::subscriber::set_default(sub);
```
