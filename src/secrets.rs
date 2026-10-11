use anyhow::{Context, Result};
use zeroize::Zeroizing;

const SERVICE: &str = "cmdpal";
const ACCOUNT: &str = "ai_api_key";

pub fn store(key: &str) -> Result<()> {
    entry()?
        .set_password(key)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .context("could not write to the OS credential store")
}

pub fn load() -> Option<Zeroizing<String>> {
    entry().ok()?.get_password().ok().map(Zeroizing::new)
}

pub fn available() -> bool {
    match entry() {
        Ok(entry) => entry.get_password().is_ok() || is_no_entry(&entry),
        Err(_) => false,
    }
}

fn is_no_entry(entry: &keyring::Entry) -> bool {
    matches!(
        entry.get_password(),
        Err(keyring::Error::NoEntry) | Err(keyring::Error::PlatformFailure(_))
    )
}

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).context("no OS credential store available")
}
