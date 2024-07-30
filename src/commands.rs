#![cfg_attr(not(feature = "commands"), allow(unused_imports))]
use crate::config::Config;
use async_scoped::spawner::use_tokio::Tokio;
use async_scoped::{Scope, TokioScope};
use chrono::Local;
use chrono_humanize::HumanTime;
use slack_morphism::events::{SlackCommandEvent, SlackCommandEventResponse};
use slack_morphism::listener::{SlackClientEventsListenerEnvironment, SlackClientEventsUserState};
use slack_morphism::prelude::{HttpStatusCode, SlackHyperClient};
use slack_morphism::{
    SlackClientSocketModeConfig, SlackClientSocketModeListener, SlackMessageContent,
    SlackMessageResponseType, SlackSocketModeListenerCallbacks, UserCallbackResult,
};
use std::sync::Arc;
use tracing::{debug, info, instrument, trace, warn};

#[cfg(feature = "commands")]
pub fn init<'a>(
    cfg: Arc<Config>,
    client: Arc<SlackHyperClient>,
) -> [(Scope<'a, (), Tokio>, ()); 1] {
    let callbacks = SlackSocketModeListenerCallbacks::new().with_command_events(handle_commands);
    let listener_env = Arc::new(
        SlackClientEventsListenerEnvironment::new(client)
            .with_user_state(cfg.clone())
            .with_error_handler(handle_errors),
    );
    let listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_env.clone(),
        callbacks,
    );

    [unsafe {
        TokioScope::scope(move |s: &mut Scope<'_, (), Tokio>| {
            s.spawn_cancellable(
                async move {
                    listener
                        .listen_for(&cfg.socket_token)
                        .await
                        .expect("Failed to initialise socket");
                    info!("listening for commands");
                    listener.serve().await;
                },
                || (),
            )
        })
    }]
}

#[cfg(not(feature = "commands"))]
#[inline]
pub fn init<'a>(_: Arc<Config>, _: Arc<SlackHyperClient>) -> [(Scope<'a, (), Tokio>, ()); 0] {
    []
}

#[instrument(skip_all)]
fn handle_errors(
    err: Box<dyn std::error::Error + Send + Sync>,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> HttpStatusCode {
    warn!("{:?}", err);

    HttpStatusCode::OK
}

#[instrument(skip_all, fields(cmd = event.command.0))]
async fn handle_commands(
    event: SlackCommandEvent,
    _client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> UserCallbackResult<SlackCommandEventResponse> {
    debug!("command received");
    Ok(match event.command.0.as_str() {
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
            trace!(next = next);
            SlackCommandEventResponse::new(SlackMessageContent::new().with_text(next))
                .with_response_type(SlackMessageResponseType::InChannel)
        }
        _ => SlackCommandEventResponse::new(
            SlackMessageContent::new().with_text("Dunno that one".to_string()),
        ),
    })
}
