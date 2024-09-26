use crate::config::Config;
#[cfg(feature = "giphy")]
use crate::giphy::Giphy;
use anyhow::Result;
use rand::prelude::IteratorRandom;
use slack_morphism::SlackMessageContent;
use std::borrow::Cow;
use tracing::info;

pub struct MessageBuilder<'a> {
    cfg: &'a Config,

    #[cfg(feature = "giphy")]
    gifs: Giphy<'a>,
}

impl<'a> MessageBuilder<'a> {
    #[cfg(not(feature = "giphy"))]
    pub fn new(cfg: &'a Config) -> MessageBuilder<'a> {
        MessageBuilder { cfg }
    }

    #[cfg(feature = "giphy")]
    pub fn new(cfg: &'a Config) -> MessageBuilder<'a> {
        MessageBuilder {
            cfg,
            gifs: Giphy::new(&cfg.giphy_token),
        }
    }

    #[cfg(not(feature = "giphy"))]
    pub async fn build_message(&self) -> Result<SlackMessageContent> {
        let msg = self.get_message();
        info!(msg, "sending");
        Ok(SlackMessageContent::new().with_text(msg.to_string()))
    }

    #[cfg(feature = "giphy")]
    pub async fn build_message(&self) -> Result<SlackMessageContent> {
        use slack_morphism::blocks::{
            SlackBlock, SlackBlockPlainTextOnly, SlackHeaderBlock, SlackImageBlock,
        };
        use url::Url;

        let search = self
            .cfg
            .gif_searches
            .iter()
            .choose(&mut rand::thread_rng())
            .unwrap();
        let gif = self.gifs.random(search).await?;

        info!(?gif, search, "sending");

        let alt = if gif.alt_text.is_empty() {
            Cow::Borrowed(search)
        } else {
            Cow::Owned(gif.alt_text)
        };

        let content = SlackMessageContent::new().with_blocks(vec![
            SlackBlock::Header(SlackHeaderBlock::new(SlackBlockPlainTextOnly::from(
                self.get_message().clone(),
            ))),
            SlackBlock::Image(
                SlackImageBlock::new(Url::parse(&gif.url)?, alt.into_owned())
                    .with_title("Powered By GIPHY".into()),
            ),
        ]);

        Ok(content)
    }

    fn get_message(&self) -> &String {
        self.cfg
            .messages
            .iter()
            .choose(&mut rand::thread_rng())
            .unwrap()
    }
}
