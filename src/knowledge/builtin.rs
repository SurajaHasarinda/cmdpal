use super::Command;

const COMPRESSED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/commands.bin"));

pub fn load_builtin() -> anyhow::Result<Vec<Command>> {
    let json = lz4_flex::decompress_size_prepended(COMPRESSED)?;
    Ok(serde_json::from_slice(&json)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn embedded_catalog_is_consistent() {
        let commands = load_builtin().unwrap();
        let ids: HashSet<_> = commands.iter().map(|c| &c.id).collect();
        assert_eq!(ids.len(), commands.len());
        assert!(commands.iter().filter(|c| c.mode == "windows").count() >= 90);
        assert!(commands.iter().all(|c| ["windows", "linux", "git", "kube", "docker"].contains(&c.mode.as_str())));
    }
}
