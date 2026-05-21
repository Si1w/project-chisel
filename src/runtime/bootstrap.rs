use std::path::Path;
use std::{collections::HashMap, fs};

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;

use crate::ecs::schedule::Schedule;
use crate::ecs::world::World;
use crate::event::bus::{Bus, BusEndpoints};
use crate::event::payload::DomainEvent;
use crate::event::queue::EventQueue;
use crate::physics::aabb::AabbEngine;
use crate::physics::gravity::Gravity;
use crate::runtime::rules::load::load_rules;
use crate::runtime::rules::system::RuleProcessor;
use crate::runtime::schema::entity::EntitySchema;
use crate::runtime::schema::game::GameSchema;
use crate::runtime::schema::paths::ManifestPaths;
use crate::runtime::schema::scene::{SceneEntitySchema, SceneSchema};
use crate::runtime::template::{Template, TemplateStore, spawn_template};
use crate::tag::set::TagSet;

const SUPPORTED_SCHEMA_VERSION: u32 = 1;
const AABB_ENGINE_NAME: &str = "aabb";
const INBOUND_BUS_CAPACITY: usize = 64;
const OUTBOUND_BUS_CAPACITY: usize = 64;
const POSITION_COMPONENT: &str = "position";
const VELOCITY_COMPONENT: &str = "velocity";
const AABB_COMPONENT: &str = "aabb";
const ANIMATOR_COMPONENT: &str = "animator";
const EVENT_TYPE_FIELD: &str = "type";

/// Everything the engine needs to start ticking. Returned by `bootstrap`;
/// also reconstructed by `command:reload`.
///
/// Per-tick flow at the runtime layer:
/// 1. `schedule.tick(&mut world, ctx)` — ECS systems only; emits to
///    `EventQueue` resource on `world`.
/// 2. `rule_processor.process(&mut world, &bus, &ctx, MAX)` — drains
///    the queue, fans out to rules, forwards events to `bus`.
pub struct EngineState {
    pub paths: ManifestPaths,
    pub world: World,
    pub schedule: Schedule,
    pub bus: Bus,
    pub bus_endpoints: BusEndpoints,
    pub rule_processor: RuleProcessor,
}

/// Reads all `.toml` files under `root`, validates them, builds the
/// initial engine state, and returns it ready for the first
/// `Schedule::tick`.
///
/// # Errors
///
/// Any failure — missing `game.toml`, unknown action name, schema
/// version mismatch, malformed component, unknown template reference —
/// surfaces as `anyhow::Error` with file/rule context. No partial state.
///
/// Resources inserted into the returned `World`:
/// - `TagRegistry` (auto, via `World::new`)
/// - `EventQueue` (auto, via `World::new`)
/// - `Gravity` (from `[physics.gravity]` in `game.toml`, if present)
/// - `TemplateStore` (populated from `entities/*.toml`)
pub fn bootstrap(root: &Path) -> Result<EngineState> {
    let paths = ManifestPaths::from_root(root);
    let game = load_game(&paths.game_toml)?;
    let mut schedule = Schedule::new();
    match game.physics.engine.as_str() {
        AABB_ENGINE_NAME => {
            schedule.add(Box::new(AabbEngine::new()));
        }
        other => bail!("unsupported physics engine {other:?}"),
    }

    let mut world = World::new();
    if let Some(gravity) = game.physics.gravity {
        world.insert_resource(Gravity(gravity));
    }

    let templates = load_templates(&paths.entities_dir, &mut world)?;
    let scene = load_scene(&paths.scenes_dir.join(format!("{}.toml", game.main_scene)))?;
    if scene.name != game.main_scene {
        bail!(
            "scene file for {:?} declared scene {:?}",
            game.main_scene,
            scene.name
        );
    }
    spawn_scene(&mut world, scene, &templates)?;
    world.insert_resource(templates);

    let rules = load_rules(&paths.rules_dir, &mut world)?;
    let rule_processor = RuleProcessor::new(rules);
    let (bus, bus_endpoints) = Bus::new(INBOUND_BUS_CAPACITY, OUTBOUND_BUS_CAPACITY);

    Ok(EngineState {
        paths,
        world,
        schedule,
        bus,
        bus_endpoints,
        rule_processor,
    })
}

fn load_game(path: &Path) -> Result<GameSchema> {
    let game = load_toml::<GameSchema>(path)?;
    if game.schema_version != SUPPORTED_SCHEMA_VERSION {
        bail!(
            "unsupported schema_version {}; expected {}",
            game.schema_version,
            SUPPORTED_SCHEMA_VERSION
        );
    }
    Ok(game)
}

fn load_templates(dir: &Path, world: &mut World) -> Result<TemplateStore> {
    let mut files = fs::read_dir(dir)
        .with_context(|| format!("read entities directory {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("read entity directory entries {}", dir.display()))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect::<Vec<_>>();
    files.sort();

    let mut templates = TemplateStore::new();
    for path in files {
        let schema = load_toml::<EntitySchema>(&path)?;
        let template = template_from_schema(schema, world)
            .with_context(|| format!("load {}", path.display()))?;
        if templates.get(&template.name).is_some() {
            bail!("duplicate template {:?}", template.name);
        }
        templates.insert(template);
    }

    Ok(templates)
}

fn load_scene(path: &Path) -> Result<SceneSchema> {
    load_toml(path)
}

fn load_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let source =
        fs::read_to_string(path).with_context(|| format!("read manifest {}", path.display()))?;
    toml::from_str(&source).with_context(|| format!("parse manifest {}", path.display()))
}

fn template_from_schema(schema: EntitySchema, world: &mut World) -> Result<Template> {
    let mut template = Template {
        name: schema.name,
        tags: tags_from_names(world, &schema.tags)?,
        ..Template::default()
    };

    apply_components(&mut template, &schema.components)?;

    Ok(template)
}

fn tags_from_names(world: &mut World, names: &[String]) -> Result<TagSet> {
    let mut tags = TagSet::new();
    for name in names {
        tags.insert(world.tag_registry_mut().intern(name)?);
    }
    Ok(tags)
}

fn spawn_scene(world: &mut World, scene: SceneSchema, templates: &TemplateStore) -> Result<()> {
    for entity in scene.entities {
        spawn_scene_entity(world, &entity, templates)?;
    }

    let initial_events = scene
        .initial_events
        .into_iter()
        .map(domain_event_from_value)
        .collect::<Result<Vec<_>>>()?;
    let queue = world
        .resource_mut::<EventQueue>()
        .expect("EventQueue is inserted by World::new");
    for event in initial_events {
        queue.emit_domain(event);
    }

    Ok(())
}

fn spawn_scene_entity(
    world: &mut World,
    scene_entity: &SceneEntitySchema,
    templates: &TemplateStore,
) -> Result<()> {
    let mut template = templates
        .get(&scene_entity.template)
        .with_context(|| format!("unknown template {:?}", scene_entity.template))?
        .clone();
    apply_components(&mut template, &scene_entity.overrides)?;
    spawn_template(world, template)?;

    Ok(())
}

fn apply_components(
    template: &mut Template,
    components: &HashMap<String, JsonValue>,
) -> Result<()> {
    for (name, value) in components {
        match name.as_str() {
            POSITION_COMPONENT => {
                template.position = Some(component_from_value(name, value)?);
            }
            VELOCITY_COMPONENT => {
                template.velocity = Some(component_from_value(name, value)?);
            }
            AABB_COMPONENT => {
                template.aabb = Some(component_from_value(name, value)?);
            }
            ANIMATOR_COMPONENT => {
                template.animator = Some(component_from_value(name, value)?);
            }
            other => bail!("unknown component {other:?}"),
        }
    }

    Ok(())
}

fn component_from_value<T: DeserializeOwned>(name: &str, value: &JsonValue) -> Result<T> {
    serde_json::from_value(value.clone()).with_context(|| format!("load component {name:?}"))
}

fn domain_event_from_value(value: JsonValue) -> Result<DomainEvent> {
    let JsonValue::Object(mut payload) = value else {
        bail!("initial event must be an object");
    };
    let JsonValue::String(name) = payload.remove(EVENT_TYPE_FIELD).with_context(|| {
        format!("initial event object must contain string field {EVENT_TYPE_FIELD:?}")
    })?
    else {
        bail!("initial event field {EVENT_TYPE_FIELD:?} must be a string");
    };

    Ok(DomainEvent::custom(name, payload))
}

#[cfg(test)]
mod tests {
    use crate::component::spatial::Velocity;
    use crate::ecs::entity::Entity;
    use crate::ecs::schedule::TickContext;
    use crate::runtime::template::TemplateStore;
    use crate::tag::set::TagSet;

    use super::*;

    #[test]
    fn bootstrap_loads_ball_collision_example() {
        let mut state =
            bootstrap(Path::new("example/ball_collision")).expect("example should bootstrap");

        assert_eq!(
            state
                .world
                .resource::<TemplateStore>()
                .expect("templates should be inserted")
                .len(),
            2
        );
        assert_eq!(state.rule_processor.rules().len(), 1);

        let ball = entity_with_tag(&state.world, "Ball");
        assert_eq!(
            state
                .world
                .get::<Velocity>(ball)
                .map(|velocity| velocity.0.x),
            Some(3.0)
        );

        let ctx = TickContext { tick: 1, dt: 0.5 };
        state.schedule.tick(&mut state.world, ctx);
        state
            .rule_processor
            .process(&mut state.world, &state.bus, &ctx, 16);

        assert_eq!(
            state
                .world
                .get::<Velocity>(ball)
                .map(|velocity| velocity.0.x),
            Some(-3.0)
        );
    }

    fn entity_with_tag(world: &World, tag: &str) -> Entity {
        let tag = world
            .tag_registry()
            .lookup(tag)
            .expect("tag should be interned");
        world
            .entities()
            .find(|entity| {
                world
                    .get::<TagSet>(*entity)
                    .is_some_and(|tags| tags.contains(tag))
            })
            .expect("entity with tag should exist")
    }
}
