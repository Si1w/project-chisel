use std::path::PathBuf;

/// Filesystem layout of a chisel project root. Pure path derivation —
/// existence and content checks live in `bootstrap`.
#[derive(Debug, Clone)]
pub struct ManifestPaths {
    pub root: PathBuf,
    pub game_toml: PathBuf,
    pub entities_dir: PathBuf,
    pub scenes_dir: PathBuf,
    pub rules_dir: PathBuf,
    pub input_toml: PathBuf,
}

impl ManifestPaths {
    #[must_use]
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            game_toml: root.join("game.toml"),
            entities_dir: root.join("entities"),
            scenes_dir: root.join("scenes"),
            rules_dir: root.join("rules"),
            input_toml: root.join("input.toml"),
            root,
        }
    }
}
