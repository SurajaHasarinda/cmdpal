use super::{prompt, sanitize::sanitize, wire::Message, Client, Error};
use crate::{config::Config, context::ContextSnapshot, modes::ActiveMode};
use anyhow::Result;
use zeroize::Zeroizing;

/// Prebuilt request; owns the key so no caller has to keep it alive across a thread boundary.
pub struct Prompt {
    key: Zeroizing<String>,
    messages: Vec<Message>,
}

impl Prompt {
    pub fn send(self, client: &Client) -> Result<String, Error> {
        let raw = client.chat(&self.key, &self.messages)?;
        sanitize(&raw).ok_or(Error::Unusable)
    }
}

pub fn prompt_for(
    config: &Config,
    query: String,
    snapshot: &ContextSnapshot,
    mode: Option<ActiveMode>,
) -> Option<Prompt> {
    let key = crate::secrets::load()?;
    let system = if config.ai.system_prompt.is_empty() {
        prompt::DEFAULT_SYSTEM_PROMPT.to_string()
    } else {
        config.ai.system_prompt.clone()
    };
    Some(Prompt {
        key,
        messages: vec![
            Message::system(system),
            Message::user(prompt::user_turn(&query, snapshot, mode, config)),
        ],
    })
}

pub fn ask(
    config: &Config,
    query: &str,
    snapshot: &ContextSnapshot,
    mode: Option<ActiveMode>,
) -> Result<String> {
    let prompt = prompt_for(config, query.to_string(), snapshot, mode).ok_or(Error::MissingKey)?;
    let client = Client::from_config(config)?;
    prompt.send(&client).map_err(Into::into)
}
