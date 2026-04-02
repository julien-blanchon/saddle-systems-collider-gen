# Collider Gen Lab

Crate-local standalone lab app for validating the shared `saddle-systems-collider-gen` crate in a real Bevy application.

## Purpose

- verify raw contour extraction, simplification, convex pieces, thresholding, atlas slicing, composite authoring, and dirty-region regeneration in one place
- expose a BRP-queryable `LabDiagnostics` resource so runtime counts can be inspected without reading internal source code
- provide crate-local `bevy_e2e` scenarios for visual and behavioral verification without relying on project-level sandboxes

## Status

Working

## Run

```bash
cargo run -p saddle-systems-collider-gen-lab
```

Keyboard shortcuts:

- `1`: overview
- `2`: thresholds
- `3`: atlas
- `4`: composite
- `5`: destructible

## E2E Scenarios

```bash
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_smoke
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_thresholds
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_atlas
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_composite
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_destructible
```

## BRP

The default `dev` feature enables `bevy_brp_extras`, so BRP workflows can launch the lab directly:

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch saddle-systems-collider-gen-lab
```
