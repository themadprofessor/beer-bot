#![cfg_attr(not(feature = "commands"), allow(unused_imports))]
use crate::config::Config;
use crate::handle_commands;
use async_scoped::spawner::use_tokio::Tokio;
use async_scoped::{Scope, TokioScope};
use slack_morphism::listener::SlackClientEventsListenerEnvironment;
use slack_morphism::prelude::SlackHyperClient;
use slack_morphism::{
    SlackClientSocketModeConfig, SlackClientSocketModeListener, SlackSocketModeListenerCallbacks,
};
use std::sync::Arc;

#[cfg(feature = "commands")]
pub fn init<'a>(
    cfg: Arc<Config>,
    client: Arc<SlackHyperClient>,
) -> [(Scope<'a, (), Tokio>, ()); 1] {
    let callbacks = SlackSocketModeListenerCallbacks::new().with_command_events(handle_commands);
    let listener_env =
        Arc::new(SlackClientEventsListenerEnvironment::new(client).with_user_state(cfg.clone()));
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
                    listener.start().await
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
