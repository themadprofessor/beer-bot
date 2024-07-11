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
                    SlackMessageContent::new().with_text(format!("@here {}", msg.clone())),
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
