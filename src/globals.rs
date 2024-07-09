use slack_morphism::hyper_tokio::{SlackClientHyperHttpsConnector, SlackHyperClient};
use slack_morphism::{SlackClient, SlackClientSession};
use tokio::sync::OnceCell;

use crate::config::Config;

static CLIENT: OnceCell<SlackHyperClient> = OnceCell::const_new();
static CONFIG: OnceCell<Config> = OnceCell::const_new();

pub async fn client() -> &'static SlackHyperClient {
    CLIENT
        .get_or_init(|| async {
            SlackClient::new(
                SlackClientHyperHttpsConnector::new().expect("Failed to initialise HTTPs client"),
            )
        })
        .await
}

pub async fn config() -> &'static Config {
    CONFIG
        .get_or_init(|| async { Config::new().await.expect("Unable to load config") })
        .await
}

pub async fn open_session() -> SlackClientSession<'static, SlackClientHyperHttpsConnector> {
    return client().await.open_session(&config().await.token);
}
