# Baking

`saddle-systems-collider-gen` supports both runtime generation and pre-baked workflows.

## When To Bake

Bake authored content when:

- the mask is static or changes only during development
- startup time matters
- you want deterministic cache keys or snapshot-friendly artifacts
- convex decomposition cost is too high to repeat on every launch

Keep runtime generation for:

- editor tools
- examples
- low-frequency destructible terrain
- iteration-heavy content previews

## Serde Workflow

Enable the `serde` feature:

```toml
saddle-systems-collider-gen = { workspace = true, features = ["serde"] }
```

Then serialize `ColliderGenResult` directly:

```rust
let result = saddle_systems_collider_gen::generate_collider_geometry(&mask, &config)?;
let bytes = serde_json::to_vec(&result)?;
```

Recommended bake payload contents:

- serialized `ColliderGenResult`
- source mask checksum or asset hash
- `ColliderGenConfig`
- crate version or your own bake schema version

## Cache Versioning

Invalidate baked data whenever any of these change:

- source mask pixels
- atlas region layout
- threshold config
- contour mode
- simplification settings
- scale
- decomposition settings
- crate version if your pipeline relies on exact numeric output

The simplest rule is:

```text
cache_key = hash(source_pixels + region + config + bake_schema_version)
```

## Recommended Production Pattern

1. Load or generate the mask in an offline tool or build step.
2. Run `generate_collider_geometry`.
3. Serialize the result to a sidecar asset.
4. Load the baked result at runtime and hand it to your physics backend adapter.

This keeps the shared crate generic:

- the bake step owns geometry generation
- your game owns the final conversion into engine-specific collider types

## Current Pass Note

Hole-aware convex decomposition is not yet the strongest part of the crate. If a baked asset has important interior voids, treat `contours + topology` as canonical and let your downstream adapter handle holes directly.
