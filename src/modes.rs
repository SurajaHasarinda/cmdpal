use clap::ValueEnum;

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub enum ActiveMode {
    Windows,
    Linux,
    Git,
    Kube,
    Docker,
}

impl ActiveMode {
    // macOS has no catalog of its own: its everyday commands match the Linux ones.
    pub const fn for_current_os() -> Self {
        if cfg!(windows) {
            ActiveMode::Windows
        } else {
            ActiveMode::Linux
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::from_str(id, true).ok()
    }

    pub fn id(self) -> &'static str {
        match self {
            ActiveMode::Windows => "windows",
            ActiveMode::Linux => "linux",
            ActiveMode::Git => "git",
            ActiveMode::Kube => "kube",
            ActiveMode::Docker => "docker",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ActiveMode::Windows => ActiveMode::Linux,
            ActiveMode::Linux => ActiveMode::Git,
            ActiveMode::Git => ActiveMode::Kube,
            ActiveMode::Kube => ActiveMode::Docker,
            ActiveMode::Docker => ActiveMode::Windows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_id_round_trips_every_mode() {
        let mut mode = ActiveMode::Windows;
        for _ in 0..5 {
            assert_eq!(ActiveMode::from_id(mode.id()), Some(mode));
            mode = mode.next();
        }
        assert_eq!(mode, ActiveMode::Windows);
        assert_eq!(ActiveMode::from_id("nonsense"), None);
    }
}
