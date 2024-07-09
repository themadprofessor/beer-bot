use anyhow::{Context, Result};
use rand::prelude::IteratorRandom;
use slack_morphism::prelude::*;
use tracing::{info, instrument};

use crate::schedule::Job;

mod config;
mod globals;
mod schedule;

#[cfg(feature = "syslog")]
fn init_log() {
    use std::ffi::CStr;
    use syslog_tracing::Syslog;
    tracing_subscriber::fmt()
        .with_writer(
            Syslog::new(
                CStr::from_bytes_with_nul(b"beerbot").unwrap(),
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        )
        .init();
}

#[cfg(not(feature = "syslog"))]
fn init_log() {
    tracing_subscriber::fmt::init();
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log();

    for schedule in &globals::config().await.crons {
        Job::new(schedule).start(move || async {
            let session = globals::open_session().await;
            let config = globals::config().await;
            let msg = {
                config
                    .messages
                    .iter()
                    .choose(&mut rand::thread_rng())
                    .unwrap()
            };
            session
                .chat_post_message(&SlackApiChatPostMessageRequest::new(
                    config.channel_id.clone(),
                    SlackMessageContent::new().with_text(msg.clone()),
                ))
                .await
                .expect("Failed to send message");

            info!(msg, "Sent message");
        });
    }

    info!("Beer Bot is ready");

    tokio::signal::ctrl_c()
        .await
        .with_context(|| "Failed to wait for ctrl+c")?;

    Ok(())
}
