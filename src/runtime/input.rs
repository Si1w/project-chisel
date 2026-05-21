use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::ecs::entity::Entity;
use crate::ecs::world::World;
use crate::event::payload::{DomainEvent, InputEvent};
use crate::runtime::rules::model::{EntityMatch, MatchSpec};
use crate::runtime::schema::input::{
    InputEmitSchema, InputMapSchema, InputSchema, InputTriggerSchema,
};
use crate::runtime::schema::rule::EntityMatchSchema;
use crate::tag::set::TagSet;

/// Translates raw player input into domain events using `input.toml`.
pub struct InputMapper {
    maps: Vec<InputMap>,
}

impl InputMapper {
    #[must_use]
    fn new(maps: Vec<InputMap>) -> Self {
        Self { maps }
    }

    /// Convert one raw input event into zero or more domain events.
    ///
    /// # Errors
    ///
    /// Returns an error if an emitted payload references an unknown match
    /// binding or if payload substitution produces a non-object payload.
    pub fn map(&self, world: &World, input: &InputEvent) -> Result<Vec<DomainEvent>> {
        let mut events = Vec::new();
        for map in self
            .maps
            .iter()
            .filter(|mapping| mapping.trigger.matches(input))
        {
            let Some(bindings) = bind_world(world, &map.match_spec) else {
                continue;
            };
            let payload = substitute_payload(&map.emit.payload, &bindings)?;
            let JsonValue::Object(payload) = payload else {
                bail!("input emit payload must be an object");
            };
            events.push(DomainEvent::custom(map.emit.event.clone(), payload));
        }

        Ok(events)
    }
}

struct InputMap {
    trigger: InputTrigger,
    match_spec: MatchSpec,
    emit: InputEmit,
}

enum InputTrigger {
    KeyPress { key: String },
    KeyRelease { key: String },
}

impl InputTrigger {
    fn matches(&self, input: &InputEvent) -> bool {
        match (self, input) {
            (Self::KeyPress { key: expected }, InputEvent::KeyPress { key })
            | (Self::KeyRelease { key: expected }, InputEvent::KeyRelease { key }) => {
                expected == key
            }
            (Self::KeyPress { .. } | Self::KeyRelease { .. }, _) => false,
        }
    }
}

struct InputEmit {
    event: String,
    payload: JsonValue,
}

struct InputBindings {
    params: Vec<String>,
    entities: Vec<Entity>,
}

impl InputBindings {
    fn entity_by_name(&self, name: &str) -> Option<Entity> {
        let index = self.params.iter().position(|param| param == name)?;
        self.entities.get(index).copied()
    }
}

/// Load `input.toml` into an `InputMapper`. Missing files are valid and
/// produce an empty mapper.
///
/// # Errors
///
/// Returns an error if `input.toml` cannot be read, parsed, or converted
/// into runtime input maps.
pub fn load_input(path: &Path, world: &mut World) -> Result<InputMapper> {
    if !path.exists() {
        return Ok(InputMapper::new(Vec::new()));
    }

    let source =
        fs::read_to_string(path).with_context(|| format!("read input file {}", path.display()))?;
    let schema = toml::from_str::<InputSchema>(&source)
        .with_context(|| format!("parse input file {}", path.display()))?;
    let maps = schema
        .maps
        .into_iter()
        .enumerate()
        .map(|(index, map)| {
            convert_input_map(map, world).with_context(|| format!("load input map {index}"))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(InputMapper::new(maps))
}

fn convert_input_map(schema: InputMapSchema, world: &mut World) -> Result<InputMap> {
    Ok(InputMap {
        trigger: convert_trigger(schema.trigger),
        match_spec: convert_match_spec(&schema.match_spec, world)?,
        emit: convert_emit(schema.emit)?,
    })
}

fn convert_trigger(schema: InputTriggerSchema) -> InputTrigger {
    match schema {
        InputTriggerSchema::KeyPress { key } => InputTrigger::KeyPress { key },
        InputTriggerSchema::KeyRelease { key } => InputTrigger::KeyRelease { key },
    }
}

fn convert_emit(schema: InputEmitSchema) -> Result<InputEmit> {
    Ok(InputEmit {
        event: schema.event_type,
        payload: object_payload(schema.payload)?,
    })
}

fn convert_match_spec(
    match_spec: &HashMap<String, EntityMatchSchema>,
    world: &mut World,
) -> Result<MatchSpec> {
    let mut params = match_spec.keys().cloned().collect::<Vec<_>>();
    params.sort();

    if params.len() > usize::from(u8::MAX) {
        bail!(
            "input map has {} match params; max is {}",
            params.len(),
            u8::MAX
        );
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

fn object_payload(payload: JsonValue) -> Result<JsonValue> {
    match payload {
        JsonValue::Null => Ok(JsonValue::Object(JsonMap::default())),
        JsonValue::Object(_) => Ok(payload),
        other => bail!("input emit payload must be an object, got {other}"),
    }
}

fn bind_world(world: &World, match_spec: &MatchSpec) -> Option<InputBindings> {
    let mut params = Vec::with_capacity(match_spec.params.len());
    let mut entities = Vec::with_capacity(match_spec.params.len());

    for (index, param) in match_spec.params.iter().enumerate() {
        let filter = match_spec.filters.get(index)?;
        let entity = world
            .entities()
            .find(|entity| !entities.contains(entity) && entity_matches(world, *entity, filter))?;
        params.push(param.clone());
        entities.push(entity);
    }

    Some(InputBindings { params, entities })
}

fn entity_matches(world: &World, entity: Entity, filter: &EntityMatch) -> bool {
    let Some(tags) = world.get::<TagSet>(entity) else {
        return filter.required.is_empty();
    };

    filter.required.iter().all(|id| tags.contains(*id))
        && filter.forbidden.iter().all(|id| !tags.contains(*id))
}

fn substitute_payload(value: &JsonValue, bindings: &InputBindings) -> Result<JsonValue> {
    match value {
        JsonValue::String(text) => {
            if let Some(param) = text.strip_prefix('$') {
                let entity = bindings
                    .entity_by_name(param)
                    .with_context(|| format!("unknown input binding {text:?}"))?;
                serde_json::to_value(entity).map_err(Into::into)
            } else {
                Ok(value.clone())
            }
        }
        JsonValue::Array(items) => {
            let mut substituted = Vec::with_capacity(items.len());
            for item in items {
                substituted.push(substitute_payload(item, bindings)?);
            }
            Ok(JsonValue::Array(substituted))
        }
        JsonValue::Object(fields) => {
            let mut substituted = JsonMap::new();
            for (key, item) in fields {
                substituted.insert(key.clone(), substitute_payload(item, bindings)?);
            }
            Ok(JsonValue::Object(substituted))
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => Ok(value.clone()),
    }
}
