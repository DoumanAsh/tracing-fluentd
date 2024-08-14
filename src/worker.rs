use core::{mem, time};

use crate::{fluent, MakeWriter};

pub enum Message {
    Record(fluent::Record),
    Terminate,
}

impl Into<Message> for fluent::Record {
    #[inline(always)]
    fn into(self) -> Message {
        Message::Record(self)
    }
}

pub trait Consumer: 'static {
    fn record(&self, record: fluent::Record);
}

#[repr(transparent)]
pub struct WorkerChannel(pub(crate) crossbeam_channel::Sender<Message>);

impl Consumer for WorkerChannel {
    #[inline(always)]
    fn record(&self, record: fluent::Record) {
        let _ = self.0.send(record.into());
    }
}

pub struct ThreadWorker {
    sender: mem::ManuallyDrop<crossbeam_channel::Sender<Message>>,
    worker: mem::ManuallyDrop<std::thread::JoinHandle<()>>,
}

impl ThreadWorker {
    #[inline(always)]
    pub fn sender(&self) -> crossbeam_channel::Sender<Message> {
        mem::ManuallyDrop::into_inner(self.sender.clone())
    }

    #[inline(always)]
    pub fn stop(&self) {
        let _result = self.sender.send(Message::Terminate);
        debug_assert!(_result.is_ok());
    }
}

impl Consumer for ThreadWorker {
    #[inline(always)]
    fn record(&self, record: fluent::Record) {
        let _ = self.sender.send(record.into());
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

pub fn thread<MW: MakeWriter>(tag: &'static str, writer: MW, max_msg_record: usize) -> std::io::Result<ThreadWorker> {
    //const MAX_WAIT: time::Duration = time::Duration::from_secs(60);

    let (sender, recv) = crossbeam_channel::unbounded();
    let worker = std::thread::Builder::new().name("tracing-fluentd-worker".to_owned());

    let worker = worker.spawn(move || {
        let mut msg = fluent::Message::new(tag);
        let mut ongoing_writer = None;

        'main_loop: loop {
            //Fetch up to max_msg_record
            while msg.len() < max_msg_record {
                match recv.recv() {
                    Ok(Message::Record(record)) => msg.add(record),
                    Ok(Message::Terminate) | Err(crossbeam_channel::RecvError) => break 'main_loop
                }
            }

            //Get every extra record we can get at the current moment.
            loop {
                match recv.try_recv() {
                    Ok(Message::Record(record)) => msg.add(record),
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Ok(Message::Terminate) | Err(crossbeam_channel::TryRecvError::Disconnected) => break 'main_loop
                }
            }

            let mut writer = match ongoing_writer.take() {
                Some(writer) => writer,
                None => match writer.make() {
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
                }
            };

            match rmp_serde::encode::write(&mut writer, &msg) {
                Ok(()) => {
                    msg.clear();
                    ongoing_writer = Some(writer);
                },
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
            for _ in 0..3 {
                let mut writer = match ongoing_writer.take() {
                    Some(writer) => writer,
                    None => match writer.make() {
                        Ok(writer) => writer,
                        Err(_) => {
                            std::thread::sleep(time::Duration::from_secs(1));
                            match writer.make() {
                                Ok(writer) => writer,
                                Err(error) => {
                                    tracing::event!(tracing::Level::DEBUG, "Failed to create fluent writer {}", error);
                                    continue
                                }
                            }
                        }
                    }
                };

                if let Err(error) = rmp_serde::encode::write(&mut writer, &msg) {
                    tracing::event!(tracing::Level::INFO, "Failed to send last records to fluent server {}", error);
                    std::thread::sleep(time::Duration::from_secs(1));
                } else {
                    break;
                }
            }
        }
    })?;

    Ok(ThreadWorker {
        sender: mem::ManuallyDrop::new(sender),
        worker: mem::ManuallyDrop::new(worker),
    })

}
