# Ball Collision Demo

Minimal authoring fixture for a ball bouncing off a wall.

The intended runtime flow is:

1. `AabbEngine` moves the ball and emits a `collision` domain event.
2. `RuleProcessor` matches `collision` where `a` has tag `Ball` and `b` has tag `Wall`.
3. The rule reverses `a`'s velocity along the collision normal.

This directory follows the v0 manifest layout. It is ready for `load_rules`
and `bootstrap` once those runtime loaders are implemented.
