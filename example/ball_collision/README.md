# Ball Collision Demo

Minimal authoring fixture for a ball bouncing off a wall.

The intended runtime flow is:

1. `AabbEngine` moves the ball and emits a `collision` domain event.
2. `RuleProcessor` matches `collision` where `a` has tag `Ball` and `b` has tag `Wall`.
3. The rule reverses `a`'s velocity along the collision normal.

Run it from the repository root:

```bash
cargo run -- run example/ball_collision --dt 0.5 --max-ticks 1
```

Expected output includes one `domain` JSONL collision event.

Inspect the initial world state:

```bash
cargo run -- inspect example/ball_collision
```

Expected output includes `snapshot` JSONL for the `Ball` and `Wall` entities.
