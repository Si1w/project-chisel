use crate::component::spatial::{Aabb, Position, Velocity};
use crate::ecs::entity::Entity;
use crate::ecs::schedule::{System, TickContext};
use crate::ecs::world::World;
use crate::event::payload::DomainEvent;
use crate::event::queue::EventQueue;
use crate::math::vec2::Vec2;
use crate::physics::engine::PhysicsEngine;
use crate::physics::gravity::Gravity;

/// v0 physics: naive O(n²) AABB integrator + overlap detector. No
/// resolution — rules respond to `Collision` events.
///
/// Per-tick:
///   1. `Velocity += gravity * dt`
///   2. `Position += Velocity * dt`
///   3. Pair-scan entities with both `Position` and `Aabb`; for each
///      overlap, push `DomainEvent::collision(a, b, normal)` into
///      the `EventQueue` resource.
///
/// Reads the `Gravity` resource each tick; if absent, gravity is zero.
#[derive(Default)]
pub struct AabbEngine;

impl AabbEngine {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl System for AabbEngine {
    fn name(&self) -> &'static str {
        "physics.aabb"
    }

    fn run(&mut self, world: &mut World, ctx: &TickContext) {
        apply_gravity(world, ctx.dt);
        integrate_positions(world, ctx.dt);
        emit_collisions(world);
    }
}

impl PhysicsEngine for AabbEngine {}

#[derive(Copy, Clone)]
struct Collider {
    entity: Entity,
    position: Vec2,
    half_extents: Vec2,
}

fn apply_gravity(world: &mut World, dt: f32) {
    let gravity = world
        .resource::<Gravity>()
        .map_or(Vec2::ZERO, |gravity| gravity.0);

    for (_, velocity) in world.query_mut::<Velocity>() {
        velocity.x += gravity.x * dt;
        velocity.y += gravity.y * dt;
    }
}

fn integrate_positions(world: &mut World, dt: f32) {
    let velocities = world
        .query::<(&Position, &Velocity)>()
        .into_iter()
        .map(|(entity, (_, velocity))| (entity, velocity.0))
        .collect::<Vec<_>>();

    for (entity, velocity) in velocities {
        if let Some(position) = world.get_mut::<Position>(entity) {
            position.x += velocity.x * dt;
            position.y += velocity.y * dt;
        }
    }
}

fn emit_collisions(world: &mut World) {
    let colliders = collect_colliders(world);
    let collisions = collisions_for(&colliders);
    let queue = world
        .resource_mut::<EventQueue>()
        .expect("EventQueue is inserted by World::new");

    for (a, b, normal) in collisions {
        queue.emit_domain(DomainEvent::collision(a, b, normal));
    }
}

fn collect_colliders(world: &World) -> Vec<Collider> {
    world
        .query::<(&Position, &Aabb)>()
        .into_iter()
        .map(|(entity, (position, aabb))| Collider {
            entity,
            position: position.0,
            half_extents: aabb.half_extents,
        })
        .collect()
}

fn collisions_for(colliders: &[Collider]) -> Vec<(Entity, Entity, Vec2)> {
    let mut collisions = Vec::new();

    for (index, a) in colliders.iter().enumerate() {
        for b in &colliders[index + 1..] {
            if let Some(normal) = overlap_normal(*a, *b) {
                collisions.push((a.entity, b.entity, normal));
            }
        }
    }

    collisions
}

fn overlap_normal(a: Collider, b: Collider) -> Option<Vec2> {
    let delta_x = b.position.x - a.position.x;
    let delta_y = b.position.y - a.position.y;
    let penetration_x = a.half_extents.x + b.half_extents.x - delta_x.abs();
    let penetration_y = a.half_extents.y + b.half_extents.y - delta_y.abs();

    if penetration_x < 0.0 || penetration_y < 0.0 {
        return None;
    }

    if penetration_x <= penetration_y {
        Some(Vec2::new(axis_sign(delta_x), 0.0))
    } else {
        Some(Vec2::new(0.0, axis_sign(delta_y)))
    }
}

fn axis_sign(delta: f32) -> f32 {
    if delta < 0.0 { -1.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::physics::gravity::Gravity;

    use super::*;

    #[test]
    fn run_integrates_position_from_velocity() {
        let mut world = World::new();
        let entity = world
            .spawn()
            .with(Position(Vec2::new(1.0, 2.0)))
            .with(Velocity(Vec2::new(3.0, -4.0)))
            .finish();
        let mut engine = AabbEngine::new();

        engine.run(&mut world, &TickContext { tick: 1, dt: 0.5 });

        assert_eq!(
            world.get::<Position>(entity).map(|position| position.0),
            Some(Vec2::new(2.5, 0.0))
        );
    }

    #[test]
    fn run_applies_gravity_before_integrating_position() {
        let mut world = World::new();
        world.insert_resource(Gravity(Vec2::new(0.0, -10.0)));
        let entity = world
            .spawn()
            .with(Position(Vec2::ZERO))
            .with(Velocity(Vec2::new(1.0, 0.0)))
            .finish();
        let mut engine = AabbEngine::new();

        engine.run(&mut world, &TickContext { tick: 1, dt: 0.5 });

        assert_eq!(
            world.get::<Velocity>(entity).map(|velocity| velocity.0),
            Some(Vec2::new(1.0, -5.0))
        );
        assert_eq!(
            world.get::<Position>(entity).map(|position| position.0),
            Some(Vec2::new(0.5, -2.5))
        );
    }

    #[test]
    fn run_emits_collision_for_overlapping_aabbs() {
        let mut world = World::new();
        let a = world
            .spawn()
            .with(Position(Vec2::ZERO))
            .with(Aabb {
                half_extents: Vec2::new(1.0, 1.0),
            })
            .finish();
        let b = world
            .spawn()
            .with(Position(Vec2::new(0.5, 1.5)))
            .with(Aabb {
                half_extents: Vec2::new(1.0, 1.0),
            })
            .finish();
        let mut engine = AabbEngine::new();

        engine.run(&mut world, &TickContext { tick: 1, dt: 0.0 });

        let event = world
            .resource_mut::<EventQueue>()
            .expect("EventQueue is inserted by World::new")
            .next_domain()
            .expect("collision event should be emitted");

        assert_eq!(event.name, "collision");
        assert_eq!(
            event.payload.get("a"),
            Some(&json!({ "index": a.index, "generation": a.generation }))
        );
        assert_eq!(
            event.payload.get("b"),
            Some(&json!({ "index": b.index, "generation": b.generation }))
        );
        assert_eq!(
            event.payload.get("normal"),
            Some(&json!({ "x": 0.0, "y": 1.0 }))
        );
    }

    #[test]
    fn run_does_not_emit_collision_for_separated_aabbs() {
        let mut world = World::new();
        let _ = world
            .spawn()
            .with(Position(Vec2::ZERO))
            .with(Aabb {
                half_extents: Vec2::new(1.0, 1.0),
            })
            .finish();
        let _ = world
            .spawn()
            .with(Position(Vec2::new(3.1, 0.0)))
            .with(Aabb {
                half_extents: Vec2::new(1.0, 1.0),
            })
            .finish();
        let mut engine = AabbEngine::new();

        engine.run(&mut world, &TickContext { tick: 1, dt: 0.0 });

        assert!(
            world
                .resource::<EventQueue>()
                .expect("EventQueue is inserted by World::new")
                .is_empty()
        );
    }
}
