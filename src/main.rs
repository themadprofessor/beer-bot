use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_scoped::spawner::use_tokio::Tokio;
use async_scoped::{Scope, TokioScope};
use chrono::Local;
use cron::Schedule;
use slack_morphism::prelude::*;
use tracing::{debug, info, instrument, trace, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::message::MessageBuilder;

mod commands;
mod config;
#[cfg(feature = "giphy")]
mod giphy;
mod message;

#[cfg(feature = "syslog")]
fn init_log(cfg: &Config) {
    use std::ffi::CStr;
    use syslog_tracing::Syslog;
    tracing_subscriber::fmt()
        .with_writer(
            Syslog::new(
                CStr::from_bytes_with_nul(b"beerbot\0").unwrap(),
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        )
        .with_env_filter(EnvFilter::new(&cfg.log))
        .init();
}

#[cfg(not(feature = "syslog"))]
fn init_log(cfg: &Config) {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cfg.log))
        .init();
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let cfg = Arc::new(
        Config::new()
            .await
            .with_context(|| "Unable to load config")?,
    );

    init_log(&cfg);

    debug!(config = %cfg);

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to initialise TLS");
    let client = Arc::new(SlackClient::new(
        SlackClientHyperHttpsConnector::new().expect("Failed to initialise HTTPs client"),
    ));

    let _tasks_iter = cfg
        .crons
        .iter()
        .map(|schedule| unsafe {
            TokioScope::scope(|s: &mut Scope<'_, (), Tokio>| {
                s.spawn_cancellable(
                    async {
                        if let Err(e) =
                            spawn_schedule(schedule, &client, &cfg, MessageBuilder::new(&cfg)).await
                        {
                            warn!(?e)
                        }
                    },
                    || (),
                )
            })
        })
        .chain(commands::init(cfg.clone(), client.clone()))
        .collect::<Vec<_>>();

    info!("Beer Bot is ready");

    tokio::signal::ctrl_c()
        .await
        .with_context(|| "Failed to wait for ctrl+c")?;

    info!("Beet bot is stopping");

    Ok(())
}

#[instrument(skip_all, fields(cron = %schedule))]
async fn spawn_schedule(
    schedule: &Schedule,
    client: &SlackHyperClient,
    config: &Config,
    builder: MessageBuilder<'_>,
) -> Result<()> {
    loop {
        if let Some(next) = schedule.upcoming(Local).next() {
            let delta = next - Local::now();
            trace!(duration = %delta, "sleeping");
            tokio::time::sleep(Duration::new(
                delta.num_seconds() as u64,
                delta.num_nanoseconds().unwrap_or(0) as u32,
            ))
            .await;
            trace!("awoken");

            let session = client.open_session(&config.token);
            session
                .chat_post_message(&SlackApiChatPostMessageRequest::new(
                    config.channel_id.clone(),
                    builder.build_message().await?,
                ))
                .await
                .expect("Failed to send message");
        } else {
            bail!("unable to find next for cron. Disabling this cron.");
        }
    }
}
