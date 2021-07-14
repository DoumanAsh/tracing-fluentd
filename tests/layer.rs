use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

#[tracing::instrument]
fn test_func(arg: u8) {
    tracing::debug!(arg = arg, "test func!");
}

#[test]
fn should_flatten_events_data() {
    let layer = tracing_fluentd::Subscriber::new("rust").flatten();
    let sub = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(sub);
    tracing::info!("LOLKA");
    test_func(1);
}

#[test]
fn should_nest_events_data() {
    let layer = tracing_fluentd::Subscriber::new("rust");
    let sub = Registry::default().with(layer);

    let _guard = tracing::subscriber::set_default(sub);
    tracing::info!("LOLKA");
    test_func(1);
}
