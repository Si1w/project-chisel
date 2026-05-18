use serde::Deserialize;

use crate::math::vec2::Vec2;

/// `game.toml` top-level. Mandatory; the presence of this file marks
/// the directory as a chisel project.
#[derive(Debug, Deserialize)]
pub struct GameSchema {
    /// Manifest format version. v0 ships `1`. Loader rejects unknown
    /// values so old engines refuse forward-incompatible manifests.
    pub schema_version: u32,

    /// Scene name loaded by `bootstrap`. Must match a `SceneSchema::name`
    /// from `scenes/`. Defaults to `"main"`; CLI `chisel run --scene X`
    /// can override at startup.
    #[serde(default = "default_main_scene")]
    pub main_scene: String,

    #[serde(default)]
    pub world: WorldSchema,

    #[serde(default)]
    pub physics: PhysicsSchema,
}

#[derive(Debug, Default, Deserialize)]
pub struct WorldSchema {
    /// Ticks per second of wall-clock time. v0 step-driven mode ignores
    /// this; reserved for v1+ real-time mode.
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u32,
}

#[derive(Debug, Default, Deserialize)]
pub struct PhysicsSchema {
    /// `PhysicsEngine` selector. v0 only accepts `"aabb"`.
    #[serde(default = "default_engine_name")]
    pub engine: String,

    /// Optional gravity vector. Defaults to zero (top-down games).
    pub gravity: Option<Vec2>,
}

fn default_tick_rate() -> u32 {
    60
}

fn default_engine_name() -> String {
    "aabb".into()
}

fn default_main_scene() -> String {
    "main".into()
}
