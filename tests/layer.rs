use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

use std::fs;

#[tracing::instrument]
fn test_func(arg: u8) {
    tracing::debug!(arg2 = arg, "test func!");
}

#[track_caller]
fn create_test_writer() -> (String, impl tracing_fluentd::MakeWriter<Writer=fs::File>) {
    let location = core::panic::Location::caller();
    let file_name = format!("fluent-records-{}.fluentd", location.line());
    let name = file_name.clone();
    (name, move || {
        fs::OpenOptions::new().read(true)
                              .write(true)
                              .append(true)
                              .create(true)
                              .open(file_name.as_str())
    })
}

#[test]
fn should_flatten_events_data() {
    let (log_name, test_writer) = create_test_writer();

    let layer = tracing_fluentd::Builder::new("rust").with_writer(test_writer).flatten().layer().expect("Create layer");
    let sub = Registry::default().with(layer);

    let guard = tracing::subscriber::set_default(sub);
    tracing::info!("LOLKA");
    for idx in 0..15 {
        test_func(idx);
    }

    drop(guard);

    let mut file = fs::File::open(log_name.as_str()).expect("To open logs");
    while let Ok(Some(output)) = rmp_serde::from_read::<_, Option<rmpv::Value>>(&mut file) {
        let output = format!("{}", output);
        println!("output={}", output);
    }

    drop(file);
    let _ = fs::remove_file(log_name);
}

//#[test]
//fn should_nest_events_data() {
//    let (log_name, test_writer) = create_test_writer();
//
//    let layer = tracing_fluentd::Builder::new("rust").with_writer(test_writer).layer().expect("Create layer");
//    let sub = Registry::default().with(layer);
//
//    let guard = tracing::subscriber::set_default(sub);
//    tracing::info!("LOLKA");
//    for idx in 0..15 {
//        test_func(idx);
//    }
//
//    drop(guard);
//
//    let mut file = fs::File::open(log_name.as_str()).expect("To open logs");
//    while let Ok(Some(output)) = rmp_serde::from_read::<_, Option<rmpv::Value>>(&mut file) {
//        let output = format!("{}", output);
//        println!("output={}", output);
//    }
//
//    drop(file);
//    let _ = fs::remove_file(log_name);
//}
//
//#[test]
//fn should_use_real_fluentd_server() {
//    let layer = tracing_fluentd::Builder::new("rust").flatten().layer().expect("Create layer");
//    let sub = Registry::default().with(layer);
//    let guard = tracing::subscriber::set_default(sub);
//    tracing::info!("LOLKA");
//    for idx in 0..100 {
//        test_func(idx);
//    }
//
//    drop(guard);
//}
