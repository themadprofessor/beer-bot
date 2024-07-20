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
//! ### Logging
//!
//! By default, beer-bot uses `stdout` for its logging, but this can be changed to `syslog` by enabling the build feature `syslog`:
//! ```shell
//! cargo build --release features syslog
//! ```
//!
//! ### Docker
//! First create a config file called `config.toml`.
//! See [Options](#options).
//!
//! ```shell
//! docker compose up
//! ```
//! Build beer-bot, create a minimal image with beer-bot and run the image.
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
//! | Key        | Meaning                                                              |
//! | ---------- | -------------------------------------------------------------------- |
//! | token      | Slack bot oAuth token - Requires `chat:write` scope                  |
//! | crons      | List of cron expressions with a seconds column prepended.            |
//! | channel_id | Either the channel name without the `#` or the ID in channel details |
//! | messages   | List of messages to randomly pick from for announcements             |
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
//! messages = ["It's that time again"]
//! ```

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_scoped::spawner::use_tokio::Tokio;
use async_scoped::{Scope, TokioScope};
use chrono::Local;
use chrono_humanize::HumanTime;
use cron::Schedule;
use rand::prelude::IteratorRandom;
use slack_morphism::prelude::*;
use tracing::{info, instrument, warn};

use crate::config::Config;

mod config;

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

    let cfg = Arc::new(
        Config::new()
            .await
            .with_context(|| "Unable to load config")?,
    );

    let client = Arc::new(SlackClient::new(
        SlackClientHyperHttpsConnector::new().expect("Failed to initialise HTTPs client"),
    ));

    let callbacks = SlackSocketModeListenerCallbacks::new().with_command_events(handle_commands);
    let listener_env = Arc::new(
        SlackClientEventsListenerEnvironment::new(client.clone()).with_user_state(cfg.clone()),
    );
    let listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_env.clone(),
        callbacks,
    );

    let _tasks = cfg
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
        .chain([unsafe {
            TokioScope::scope(|s: &mut Scope<'_, (), Tokio>| {
                s.spawn_cancellable(
                    async {
                        listener
                            .listen_for(&cfg.socket_token)
                            .await
                            .expect("Failed to initialise socket");
                        listener.start().await
                    },
                    || (),
                )
            })
        }])
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

#[instrument(skip_all)]
async fn handle_commands(
    event: SlackCommandEvent,
    _client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> UserCallbackResult<SlackCommandEventResponse> {
    Ok(SlackCommandEventResponse::new(
        match event.command.0.as_str() {
            "/when-can-i-drink" => {
                let now = Local::now();
                let next = states
                    .read()
                    .await
                    .get_user_state::<Arc<Config>>()
                    .expect("Unable to get config")
                    .crons
                    .iter()
                    .filter_map(|s| s.upcoming(Local).next())
                    .map(|dt| dt - now)
                    .min()
                    .map(|d| HumanTime::from(d).to_string())
                    .unwrap_or_else(|| "in some time".to_string());
                SlackMessageContent::new().with_text(next)
            }
            _ => SlackMessageContent::new().with_text("Dunno that one".to_string()),
        },
    ))
}
