use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value as JsonValue};

use crate::ecs::world::World;
use crate::math::vec2::Vec2;
use crate::runtime::rules::model::{
    Action, EntityMatch, MatchSpec, ParamId, Rule, RuleId, RuleSet,
};
use crate::runtime::schema::rule::{ActionSchema, EntityMatchSchema, RuleSchema};

/// Loads all `*.toml` files from `dir` into a `RuleSet`. Tag names are
/// interned into the world's `TagRegistry` as they're encountered.
///
/// Files load in lexicographic order so rules sharing an event keep a
/// deterministic precedence.
///
/// # Errors
///
/// First failure short-circuits with `anyhow` context describing which
/// file / rule / action failed. v0 callers (`bootstrap`, `reload`
/// command handler) just surface the chain to stderr — no programmatic
/// recovery is expected from a malformed manifest.
pub fn load_rules(dir: &Path, world: &mut World) -> Result<RuleSet> {
    let mut rules = RuleSet::new();
    if !dir.exists() {
        return Ok(rules);
    }

    let mut files = fs::read_dir(dir)
        .with_context(|| format!("read rules directory {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("read rules directory entries {}", dir.display()))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect::<Vec<_>>();
    files.sort();

    for path in files {
        let rule = load_rule_file(&path, world)?;
        rules.add(rule);
    }

    Ok(rules)
}

fn load_rule_file(path: &Path, world: &mut World) -> Result<Rule> {
    let source =
        fs::read_to_string(path).with_context(|| format!("read rule file {}", path.display()))?;
    let schema = toml::from_str::<RuleSchema>(&source)
        .with_context(|| format!("parse rule file {}", path.display()))?;
    let id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("rule file name must be valid UTF-8")?
        .to_owned();

    convert_rule(RuleId(id), schema, world).with_context(|| format!("load rule {}", path.display()))
}

fn convert_rule(id: RuleId, schema: RuleSchema, world: &mut World) -> Result<Rule> {
    if schema.actions.is_empty() {
        bail!("rule {} must contain at least one [[do]] action", id.0);
    }

    let match_spec = convert_match_spec(&schema.match_spec, world)
        .with_context(|| format!("load match spec for rule {}", id.0))?;
    let param_ids = param_ids(&match_spec)?;
    let actions = schema
        .actions
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            convert_action(action, &param_ids)
                .with_context(|| format!("load action {index} for rule {}", id.0))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Rule {
        id,
        event_name: schema.event,
        match_spec,
        actions,
    })
}

fn convert_match_spec(
    match_spec: &HashMap<String, EntityMatchSchema>,
    world: &mut World,
) -> Result<MatchSpec> {
    let mut params = match_spec.keys().cloned().collect::<Vec<_>>();
    params.sort();

    if params.len() > usize::from(u8::MAX) {
        bail!("rule has {} match params; max is {}", params.len(), u8::MAX);
    }

    let filters = params
        .iter()
        .map(|param| {
            let schema = match_spec
                .get(param)
                .expect("param came from match spec keys");
            convert_entity_match(schema, world)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(MatchSpec { params, filters })
}

fn convert_entity_match(schema: &EntityMatchSchema, world: &mut World) -> Result<EntityMatch> {
    let required = schema
        .with
        .iter()
        .map(|tag| world.tag_registry_mut().intern(tag).map_err(Into::into))
        .collect::<Result<Vec<_>>>()?;
    let forbidden = schema
        .without
        .iter()
        .map(|tag| world.tag_registry_mut().intern(tag).map_err(Into::into))
        .collect::<Result<Vec<_>>>()?;

    Ok(EntityMatch {
        required,
        forbidden,
    })
}

fn param_ids(match_spec: &MatchSpec) -> Result<HashMap<String, ParamId>> {
    match_spec
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let id = u8::try_from(index)
                .with_context(|| format!("param index {index} does not fit in u8"))?;
            Ok((param.clone(), ParamId(id)))
        })
        .collect()
}

fn convert_action(action: ActionSchema, param_ids: &HashMap<String, ParamId>) -> Result<Action> {
    match action {
        ActionSchema::SetVelocity { entity, x, y } => Ok(Action::SetVelocity {
            target: resolve_param(param_ids, &entity)?,
            velocity: Vec2::new(x, y),
        }),
        ActionSchema::ReverseVelocity { entity, axis } => Ok(Action::ReverseVelocity {
            target: resolve_param(param_ids, &entity)?,
            axis,
        }),
        ActionSchema::Spawn { template, position } => Ok(Action::Spawn { template, position }),
        ActionSchema::Despawn { entity } => Ok(Action::Despawn {
            target: resolve_param(param_ids, &entity)?,
        }),
        ActionSchema::Emit { event, payload } => Ok(Action::Emit {
            event,
            payload: object_payload(payload)?,
        }),
        ActionSchema::PlayAnimation {
            entity,
            clip,
            priority,
        } => Ok(Action::PlayAnimation {
            target: resolve_param(param_ids, &entity)?,
            clip,
            priority,
        }),
    }
}

fn resolve_param(param_ids: &HashMap<String, ParamId>, name: &str) -> Result<ParamId> {
    param_ids
        .get(name)
        .copied()
        .with_context(|| format!("unknown match param {name:?}"))
}

fn object_payload(payload: JsonValue) -> Result<JsonValue> {
    match payload {
        JsonValue::Null => Ok(JsonValue::Object(Map::default())),
        JsonValue::Object(_) => Ok(payload),
        other => bail!("emit payload must be an object, got {other}"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::Value as JsonValue;

    use crate::runtime::rules::model::{Action, ReverseAxis};

    use super::*;

    #[test]
    fn loads_ball_collision_rule_from_example() {
        let mut world = World::new();

        let rules = load_rules(Path::new("example/ball_collision/rules"), &mut world)
            .expect("example rules should load");

        assert_eq!(rules.len(), 1);
        let collision_rules = rules.rules_for("collision");
        assert_eq!(collision_rules.len(), 1);

        let rule = &collision_rules[0];
        assert_eq!(rule.id.0, "ball-bounce");
        assert_eq!(rule.match_spec.params, vec!["a", "b"]);
        assert_eq!(rule.actions.len(), 2);
        assert!(matches!(
            &rule.actions[0],
            Action::ReverseVelocity {
                target,
                axis: ReverseAxis::FromNormal
            } if target.0 == 0
        ));
        assert!(matches!(
            &rule.actions[1],
            Action::Emit { event, payload }
                if event == "bounced"
                    && payload.pointer("/who").and_then(JsonValue::as_str) == Some("$a")
        ));

        let ball = world
            .tag_registry()
            .lookup("Ball")
            .expect("Ball tag should be interned");
        let wall = world
            .tag_registry()
            .lookup("Wall")
            .expect("Wall tag should be interned");
        assert_eq!(rule.match_spec.filters[0].required, vec![ball]);
        assert_eq!(rule.match_spec.filters[1].required, vec![wall]);
    }

    #[test]
    fn missing_rules_directory_loads_empty_rule_set() {
        let mut world = World::new();

        let rules = load_rules(Path::new("example/ball_collision/not-rules"), &mut world)
            .expect("missing optional rules directory should load");

        assert!(rules.is_empty());
    }

    #[test]
    fn rejects_rule_without_actions() {
        let dir = TempRuleDir::new("empty-actions");
        dir.write_rule(
            "empty.toml",
            r#"
event = "collision"

[match.a]
with = ["Ball"]
"#,
        );
        let mut world = World::new();

        let Err(error) = load_rules(dir.path(), &mut world) else {
            panic!("rule should be invalid");
        };

        assert!(error_contains(&error, "at least one [[do]] action"));
    }

    #[test]
    fn rejects_action_referencing_unknown_match_param() {
        let dir = TempRuleDir::new("unknown-param");
        dir.write_rule(
            "bad-param.toml",
            r#"
event = "collision"

[match.a]
with = ["Ball"]

[[do]]
action = "reverse_velocity"
entity = "b"
axis = "both"
"#,
        );
        let mut world = World::new();

        let Err(error) = load_rules(dir.path(), &mut world) else {
            panic!("rule should be invalid");
        };

        assert!(error_contains(&error, "unknown match param"));
    }

    #[test]
    fn rejects_emit_payload_that_is_not_an_object() {
        let dir = TempRuleDir::new("bad-payload");
        dir.write_rule(
            "bad-payload.toml",
            r#"
event = "collision"

[[do]]
action = "emit"
event = "bad"
payload = 7
"#,
        );
        let mut world = World::new();

        let Err(error) = load_rules(dir.path(), &mut world) else {
            panic!("rule should be invalid");
        };

        assert!(error_contains(&error, "emit payload must be an object"));
    }

    #[test]
    fn missing_emit_payload_defaults_to_empty_object() {
        let dir = TempRuleDir::new("default-payload");
        dir.write_rule(
            "emit.toml",
            r#"
event = "tick"

[[do]]
action = "emit"
event = "spawned"
"#,
        );
        let mut world = World::new();

        let rules = load_rules(dir.path(), &mut world).expect("rule should load");
        let action = &rules.rules_for("tick")[0].actions[0];

        assert!(matches!(
            action,
            Action::Emit {
                event,
                payload: JsonValue::Object(payload)
            } if event == "spawned" && payload.is_empty()
        ));
    }

    struct TempRuleDir {
        path: std::path::PathBuf,
    }

    impl TempRuleDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("chisel-{name}-{nanos}"));
            fs::create_dir_all(&path).expect("temp rule dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write_rule(&self, file_name: &str, source: &str) {
            fs::write(self.path.join(file_name), source).expect("temp rule should be written");
        }
    }

    impl Drop for TempRuleDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn error_contains(error: &anyhow::Error, expected: &str) -> bool {
        error
            .chain()
            .any(|cause| cause.to_string().contains(expected))
    }
}
