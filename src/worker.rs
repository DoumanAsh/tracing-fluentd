use core::{mem, time};

use crate::{fluent, MakeWriter};

pub trait Consumer: 'static {
    fn record(&self, record: fluent::Record);
}

pub struct ThreadWorker {
    sender: mem::ManuallyDrop<crossbeam_channel::Sender<fluent::Record>>,
    worker: mem::ManuallyDrop<std::thread::JoinHandle<()>>,
}

impl Consumer for ThreadWorker {
    #[inline(always)]
    fn record(&self, record: fluent::Record) {
        let _result = self.sender.send(record);
        debug_assert!(_result.is_ok());
    }
}

impl Drop for ThreadWorker {
    fn drop(&mut self) {
        let worker = unsafe {
            mem::ManuallyDrop::drop(&mut self.sender);
            mem::ManuallyDrop::take(&mut self.worker)
        };
        //Since we're dropping then probably application is terminating
        //or logger is removed, so no one would receive event
        let _ = worker.join();
    }
}

pub fn thread<MW: MakeWriter>(tag: &'static str, writer: MW) -> std::io::Result<ThreadWorker> {
    const MAX_MSG_RECORD: usize = 10;
    //const MAX_WAIT: time::Duration = time::Duration::from_secs(60);

    let (sender, recv) = crossbeam_channel::unbounded();
    let worker = std::thread::Builder::new().name("tracing-fluentd-worker".to_owned());

    let worker = worker.spawn(move || {
        let mut msg = fluent::Message::new(tag);

        'main_loop: loop {
            //Fetch up to MAX_MSG_RECORD
            while msg.len() < MAX_MSG_RECORD {
                match recv.recv() {
                    Ok(record) => msg.add(record),
                    Err(crossbeam_channel::RecvError) => break 'main_loop
                }
            }

            //Get every extra record we can get at the current moment.
            loop {
                match recv.try_recv() {
                    Ok(record) => msg.add(record),
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => break 'main_loop
                }
            }

            let mut writer = match writer.make() {
                Ok(writer) => writer,
                Err(_) => {
                    std::thread::sleep(time::Duration::from_secs(1));
                    match writer.make() {
                        Ok(writer) => writer,
                        Err(error) => {
                            tracing::event!(tracing::Level::DEBUG, "Failed to create fluent writer {}", error);
                            continue 'main_loop;
                        }
                    }
                }
            };

            match rmp_serde::encode::write(&mut writer, &msg) {
                Ok(()) => msg.clear(),
                //In case of error we'll just retry at later date.
                //Ideally we should be able to recover.
                //But report error?
                Err(error) => {
                    tracing::event!(tracing::Level::INFO, "Failed to send records to fluent server {}", error);
                },
            }
        }

        if msg.len() > 0 {
            //Try to flush last records, but don't wait too much
            for _ in 0..2 {
                match writer.make() {
                    Ok(mut writer) => {
                        if let Err(error) = rmp_serde::encode::write(&mut writer, &msg) {
                            tracing::event!(tracing::Level::INFO, "Failed to send last records to fluent server {}", error);
                            std::thread::sleep(time::Duration::from_secs(1));
                        } else {
                            break;
                        }
                    },
                    Err(error) => {
                        tracing::event!(tracing::Level::INFO, "Failed to create fluent server {}", error);
                        std::thread::sleep(time::Duration::from_secs(1));
                    }
                }
            }
        }

    })?;

    Ok(ThreadWorker {
        sender: mem::ManuallyDrop::new(sender),
        worker: mem::ManuallyDrop::new(worker),
    })

}
