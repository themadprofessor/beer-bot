use std::future::Future;
use std::time::Duration;

use chrono::Local;
use cron::Schedule;
use tokio::task::JoinHandle;
use tracing::{instrument, warn};

#[derive(Debug)]
pub struct Job<'a> {
    schedule: &'a Schedule,
}

impl<'a> Job<'a> {
    pub fn new(schedule: &'a Schedule) -> Job {
        Job { schedule }
    }
}

impl Job<'static> {
    #[instrument(skip_all)]
    pub fn start<T, F>(self, do_thing: T) -> JoinHandle<()>
    where
        T: (Fn() -> F) + Send + Sync + 'static,
        F: Future + Send,
    {
        tokio::spawn(async move {
            loop {
                if let Some(next) = self.schedule.upcoming(Local).next() {
                    let delta = next - Local::now();
                    tokio::time::sleep(Duration::new(
                        delta.num_seconds() as u64,
                        delta.num_nanoseconds().unwrap_or(0) as u32,
                    ))
                    .await;
                    do_thing().await;
                } else {
                    warn!("unable to find next for cron {}", self.schedule);
                    break;
                }
            }
        })
    }
}
