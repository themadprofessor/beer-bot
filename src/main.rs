//! Reminder to celebrate a (possibly) long day with a beer 🍺.
//!
//! ## Description
//! A Slack Bot to announce it is time to enjoy a beer/< insert alcohol of choice>/< insert non-alcoholic beverage of choice>.
//!
//! ## Build
//!
//! Nice and straight forward:
//! ```shell
//! cargo build --release
//! ```
//! Resulting in the binary `./target/release/beer-bot`.
//!
//!
//! ### Features
//!
//! | Feature  | Quick Explanation                       | Enabled by Default |
//! |----------|-----------------------------------------|--------------------|
//! | syslog   | Output to syslog                        | ☐                  |
//! | commands | Enable slash commands using Socket Mode | ☑                  |
//!
//! Features are additive.
//! So to have Beer Bot output to Syslog and not enable slash commands, all default features must first be disabled:
//!
//! ```shell
//! cargo build --release --no-default-features --features "syslog"
//! ```
//!
//! #### Syslog Feature
//!
//! By default, Beer Bot output to stdout, but this can be changed to utilise syslog.
//! The logging levels are mapped to syslog severities according to the following table:
//!
//! | Log Level | Syslog Severity |
//! |-----------|-----------------|
//! | ERROR     | Error           |
//! | WARN      | Warning         |
//! | INFO      | Notice          |
//! | DEBUG     | Info            |
//! | TRACE     | Debug           |
//!
//! #### Commands Feature
//!
//! With this feature disabled, beer-bot spends almost all of its time sleeping.
//! Only waking when any of the crons trigger.
//! Therefore, if slash commands are not needed, it's recommended to disable them.
//!
//! Rather than require beer-bot to act as a HTTP server and what that entails to receive slash
//! commands, it utilises [Socket Mode](https://api.slack.com/apis/socket-mode).
//! Enabling Socket Mode in Slack's App Config will generate an "App Level Token" which beer-bot
//! use to establish the web socket between it and Slack.
//! This token *must* be passed to beer-bot using the `socket_token` [option](#options).
//!
//! Once Socket Mode is enabled, a [Slash Command](https://api.slack.com/interactivity/slash-commands)
//! can be created without having to specify an endpoint.
//!
//! Beer-bot listens for the following commands:
//! * `when-can-i-drink`
//!
//! ### Docker
//! First create a config file called `config.toml`.
//! See [Options](#options).
//!
//! ```shell
//! docker compose up
//! ```
//! Build beer-bot, create a minimal image with beer-bot and run the image.
//! This file is included in the `docker-compose.yml` as a secret.
//!
//! ## Usage
//!
//! Nice and straight forward:
//! ```shell
//! beer-bot
//! ```
//!
//! beer-bot is configured by combining a config file and environment variables, where environment variables take precedence over the config file.
//! All the options need to be specified.
//!
//! ### Options
//! | Key          | Meaning                                                              |
//! | ------------ | -------------------------------------------------------------------- |
//! | token        | Slack bot oAuth token - Requires `chat:write` scope                  |
//! | socket_token | Slack SocketMode token - Only required if `commands` feature enabled |
//! | crons        | List of cron expressions with a seconds column prepended.            |
//! | channel_id   | Either the channel name without the `#` or the ID in channel details |
//! | messages     | List of messages to randomly pick from for announcements             |
//!
//! ### Environment Variables
//!
//! Environment variables are the same as the config file keys, but in `SCREAMING_SNAKE_CASE` and prefixed with `BEERBOT_`.
//! Lists like `messages` are seperated by `¬`. (Needed a symbol that isn't likely to be in the messages).
//!
//! #### Examples
//! ```shell
//! BEERBOT_TOKEN="xo..."
//! BEERBOT_CHANNEL_ID="beer-bot"
//! BEERBOT_MESSAGES="Lets Go¬Its time to party"
//! ```
//!
//! ### Config File
//! |Platform | Location                                                                         |
//! | ------- | -------------------------------------------------------------------------------- |
//! | Linux   | `$XDG_CONFIG_HOME/beerbot/beer-bot.toml` or `$HOME/.config/beerbot/beerbot.toml` |
//! | macOS   | `$HOME/Library/Application Support/com.beerbot.beerbot/beerbot.toml`             |
//! | Windows | `{FOLDERID_LocalAppData}\\com\\beerbot\\beerbot\\config\\beerbot.toml`            |
//!
//! #### Example
//! ```toml
//! token = "xo..."
//! crons = ["0 0 17 * * mon-fri *"]
//! channel_id = "beer-bot"
//! messages = ["It's that time again", "LETS GO"]
//! ```

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_scoped::spawner::use_tokio::Tokio;
use async_scoped::{Scope, TokioScope};
use chrono::Local;
use cron::Schedule;
use rand::prelude::IteratorRandom;
use slack_morphism::prelude::*;
use tracing::{info, instrument, warn};

use crate::config::Config;

mod commands;
mod config;

#[cfg(feature = "syslog")]
fn init_log() {
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

    let cfg = Arc::new(
        Config::new()
            .await
            .with_context(|| "Unable to load config")?,
    );

    let client = Arc::new(SlackClient::new(
        SlackClientHyperHttpsConnector::new().expect("Failed to initialise HTTPs client"),
    ));

    let _tasks_iter = cfg
        .crons
        .iter()
        .map(|schedule| unsafe {
            TokioScope::scope(|s: &mut Scope<'_, (), Tokio>| {
                s.spawn_cancellable(
                    async { spawn_schedule(schedule, &client, &cfg).await },
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

#[instrument(skip_all)]
async fn spawn_schedule(schedule: &Schedule, client: &SlackHyperClient, config: &Config) {
    loop {
        if let Some(next) = schedule.upcoming(Local).next() {
            let delta = next - Local::now();
            tokio::time::sleep(Duration::new(
                delta.num_seconds() as u64,
                delta.num_nanoseconds().unwrap_or(0) as u32,
            ))
            .await;

            let session = client.open_session(&config.token);
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
                    SlackMessageContent::new().with_text(format!("@everyone {}", msg.clone())),
                ))
                .await
                .expect("Failed to send message");

            info!(msg, schedule = %schedule, "sent");
        } else {
            warn!("unable to find next for cron {}", schedule);
            break;
        }
    }
}
