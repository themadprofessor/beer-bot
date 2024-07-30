use std::env;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use config::builder::AsyncState;
use config::{
    AsyncSource, ConfigBuilder, ConfigError, Environment, FileFormat, Format, Map, Value,
};
use cron::Schedule;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use slack_morphism::{SlackApiToken, SlackApiTokenValue, SlackChannelId};
use tracing::instrument;

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_token")]
    pub token: SlackApiToken,
    #[cfg(feature = "commands")]
    #[serde(deserialize_with = "deserialize_token")]
    pub socket_token: SlackApiToken,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub crons: Vec<Schedule>,
    pub channel_id: SlackChannelId,
    pub messages: Vec<String>,
    #[serde(default)]
    pub log: String,
}

#[derive(Debug)]
struct AsyncFileSource<F: Format + Debug, P: AsRef<Path> + Debug> {
    format: F,
    file: P,
}

struct SlackApiTokenVisitor;

impl Config {
    #[instrument]
    pub async fn new() -> Result<Config> {
        let mut config_builder = ConfigBuilder::<AsyncState>::default();

        if let Some(dirs) = directories::ProjectDirs::from("com", "beerbot", "beerbot") {
            let path = dirs.config_local_dir().join("beerbot.toml");
            if tokio::fs::try_exists(&path).await.unwrap_or(false) {
                config_builder = config_builder.add_async_source(AsyncFileSource {
                    format: FileFormat::Toml,
                    file: path,
                });
            } else {
                eprintln!("Config not found, skipping. {}", path.display());
            }
        }

        let tmp = env::args().nth(1);
        if let Some(cfg_path) = tmp {
            let path = PathBuf::from(cfg_path);
            if tokio::fs::try_exists(&path).await.unwrap_or(false) {
                config_builder = config_builder.add_async_source(AsyncFileSource {
                    format: FileFormat::Toml,
                    file: path,
                })
            } else {
                bail!("{} does not exist", path.display());
            }
        }

        let cfg = config_builder
            .add_source(
                Environment::with_prefix("BEERBOT")
                    .list_separator("¬")
                    .try_parsing(true)
                    .with_list_parse_key("messages")
                    .with_list_parse_key("crons"),
            )
            .build()
            .await
            .with_context(|| "Failed to load config")?;

        cfg.try_deserialize()
            .with_context(|| "Failed to convert config")
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{{ token: (len:{}), socket_token: (len:{}), crons: [{}], messages: [{}], log: \"{}\" }}",
            self.token.token_value.0.len(),
            self.socket_token.token_value.0.len(),
            self.crons
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            self.messages.join(", "),
            self.log
        ))
    }
}

// Remove async_trait when config drops it
#[async_trait]
impl<F: Format + Send + Sync + Debug, P: AsRef<Path> + Debug + Sync> AsyncSource
    for AsyncFileSource<F, P>
{
    async fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let content =
            tokio::fs::read_to_string(&self.file)
                .await
                .map_err(|e| ConfigError::FileParse {
                    uri: Some(self.file.as_ref().display().to_string()),
                    cause: Box::new(e),
                })?;

        let url = self.file.as_ref().display().to_string();
        self.format
            .parse(Some(&url), &content)
            .map_err(ConfigError::Foreign)
    }
}

impl<'de> Visitor<'de> for SlackApiTokenVisitor {
    type Value = SlackApiToken;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a Slack API token")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SlackApiToken::new(SlackApiTokenValue(v)))
    }
}

fn deserialize_token<'de, D>(deserializer: D) -> Result<SlackApiToken, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_string(SlackApiTokenVisitor)
}
