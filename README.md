# Saddle Systems Collider Gen

Engine-agnostic 2D collision geometry generation for Bevy. The crate turns binary masks, Bevy `Image` assets, atlas subregions, and procedural raster edits into reusable outline loops, topology metadata, convex hulls, and convex-piece compounds without depending on any physics backend.

## Quick Start

```toml
[dependencies]
saddle-systems-collider-gen = { git = "https://github.com/julien-blanchon/saddle-systems-collider-gen" }
```

```rust
use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, ColliderGenConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mask = BinaryImage::new(24, 16);
    mask.fill_rect(0, 0, 24, 3);
    mask.fill_rect(4, 6, 8, 2);
    mask.carve_circle(IVec2::new(18, 2), 2);

    let result = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig::default(),
    )?;

    for contour in &result.contours {
        println!("outline with {} vertices", contour.points.len());
    }
    Ok(())
}
```

If you want ECS integration instead of direct function calls:

```rust
use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, ColliderGenPlugin, ColliderGenSource, ColliderGenSourceKind};

App::new()
    .add_plugins(MinimalPlugins)
    .add_plugins(ColliderGenPlugin)
    .add_systems(Startup, |mut commands: Commands| {
        let mut mask = BinaryImage::new(16, 16);
        mask.fill_rect(0, 0, 16, 4);

        commands.spawn(ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(mask),
            config: default(),
        });
    });
```

`ColliderGenPlugin` mounts its ECS pipeline into `Update` by default. If your game wants the
crate to run in a custom schedule, use `ColliderGenPlugin::in_schedule(FixedUpdate)` or any other
schedule label that fits your state machine or tool workflow.

## Feature Flags

| Feature | Purpose |
| --- | --- |
| `image` | Enables `BinaryImage::from_dynamic_image` using the `image` crate |
| `serde` | Enables `Serialize` / `Deserialize` derives on cache-friendly pure data such as `ColliderGenResult`, configs, contours, and atlas metadata |

## Public API

### Pure data and geometry

| Type | Purpose |
| --- | --- |
| `BinaryImage` | Top-left-origin boolean mask with fill, carve, crop, invert, grow/shrink, morphology, stamping, and threshold helpers |
| `Contour` | Closed polygon loop in either pixel or local-centered space |
| `ContourTopology` | Parent / child hierarchy plus `is_hole` classification |
| `ColliderGenConfig` | Thresholding, contour mode, scale, simplification, LOD, and decomposition settings |
| `ColliderGenResult` | Final contours, topology, convex hulls, convex pieces, bounds, and warnings |
| `AtlasSlicer` | Bevy-style row-major atlas slicing from full masks or arbitrary `URect` subregions |
| `AtlasColliderFrame`, `bake_atlas_collider_frames` | Batch-bakes non-empty atlas slices into deterministic per-frame collider outputs |
| `CoordinateTransform` | Pixel-corner to local-centered coordinate conversion with non-uniform scale |

### ECS integration

| Type | Purpose |
| --- | --- |
| `ColliderGenPlugin` | Registers ECS systems, messages, and reflected component types on `Update` by default |
| `ScheduledColliderGenPlugin` | Configured plugin returned by `ColliderGenPlugin::in_schedule(...)` for custom schedule integration |
| `ColliderGenSystems` | Public ordering hooks: `Extract`, `Generate`, `Validate`, `Cache` |
| `ColliderGenSource` | ECS source component holding a binary mask or Bevy image handle plus config; intentionally not `serde`-serializable because `Handle<Image>` is runtime Bevy state |
| `ColliderGenDirty` | Optional dirty-region marker for targeted regeneration; isolated edits merge back into the previous full result, while boundary-touching crops fall back to full regeneration for correctness |
| `ColliderGenOutput` | Generated geometry plus cached summary metadata and the latest generation summary (`full rebuild`, `dirty merge`, or `dirty fallback`) |
| `ColliderGenGenerationSummary` | Explicit ECS observability for how the latest output was produced and which dirty crop was considered |
| `ColliderGenFinished` / `ColliderGenFailed` | Buffered completion messages; finished messages include the same generation summary for systems that react without re-querying the component |

## Output Modes

The crate intentionally does not emit Rapier, Avian, or Box2D colliders. It produces reusable geometry:

- `result.contours`: closed outline loops ready for chain or segment-style terrain
- `result.topology`: island / hole relationships for solid polygon workflows
- `result.convex_hulls`: conservative one-piece approximations
- `result.convex_pieces`: convex-piece compounds for downstream engines that want many convex parts

Current pass behavior:

- pixel-exact extraction is the default
- marching squares is available as an alternate contour mode
- convex decomposition follows the same final contour set that is published in `result.contours`, so LOD and contour-mode changes affect `convex_pieces` deterministically instead of bypassing the configured pipeline
- convex decomposition currently merges simple outer contours conservatively; for hole-heavy authored content, prefer `contours + topology` as the authoritative output
- convex pieces are sorted deterministically by area and local offset so bake outputs stay stable across full and dirty-region rebuilds

## Examples

From the crate root, switch into the examples workspace before running any of these packages:

```bash
cd examples
```

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal authored-mask workflow with live contour-mode, simplification, scale, and decomposition tuning | `cargo run -p saddle-systems-collider-gen-example-basic` |
| `atlas` | Atlas slicing and per-frame / per-tile generation using the `image` feature, with live extraction tuning | `cargo run -p saddle-systems-collider-gen-example-atlas` |
| `masks` | Binary-image morphology and cleanup workflows with interactive radius controls | `cargo run -p saddle-systems-collider-gen-example-masks` |
| `destructible` | ECS-driven dirty regeneration on a mutable terrain mask with live blast radius and config tuning | `cargo run -p saddle-systems-collider-gen-example-destructible` |
| `tilemap_merge` | Game-like tile composition demo that stamps reusable masks into one seamless collider canvas | `cargo run -p saddle-systems-collider-gen-example-tilemap-merge` |
| `animation_frames` | Precomputed frame geometry from a spritesheet-like atlas via `bake_atlas_collider_frames`, with live playback and extraction tuning | `cargo run -p saddle-systems-collider-gen-example-animation-frames` |
| `debug_gizmos` | Pixel-exact vs marching-squares comparison with BRP extras enabled and live simplification controls | `cargo run -p saddle-systems-collider-gen-example-debug-gizmos` |
| `perf` | Interactive benchmark dashboard for sparse+dense masks, simplification, atlas slicing, and dirty-region regen | `cargo run -p saddle-systems-collider-gen-example-perf` |

Every visual example now includes a `saddle-pane` panel that exposes the key generation settings live:

- render scale
- contour mode
- convex decomposition toggle
- simplification tolerances
- example-specific controls such as morphology radius, frame cadence, or blast radius

## Workspace Lab

This repository also provides a crate-local lab app at `examples/lab`:

```bash
cd examples
cargo run -p saddle-systems-collider-gen-lab
```

Crate-local E2E scenarios live with that lab instead of `crates/e2e`:

```bash
cd examples
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_basic
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_smoke
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_thresholds
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_atlas
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_composite
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_destructible
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_masks
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_tilemap_merge
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_debug_gizmos
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_animation_frames
cargo run -p saddle-systems-collider-gen-example-perf --features e2e -- collider_gen_perf_smoke
```

Current example-to-E2E mapping:

- `basic` -> `collider_gen_basic`
- `atlas` -> `collider_gen_atlas`
- `masks` -> `collider_gen_masks`
- `destructible` -> `collider_gen_destructible`
- `tilemap_merge` -> `collider_gen_tilemap_merge`
- `animation_frames` -> `collider_gen_animation_frames`
- `debug_gizmos` -> `collider_gen_debug_gizmos`
- `perf` -> `collider_gen_perf_smoke`

The lab also exposes the same `saddle-pane` controls so you can switch views, adjust the active
destructible config, and inspect contour / piece counts without recompiling.

## Recommended Bake Workflow

Use runtime generation for:

- tools
- one-off load screens
- examples
- rare destructible edits

Prefer baked geometry for shipped authored content:

1. Generate once from `BinaryImage` or a source `Image`.
2. Serialize `ColliderGenResult` behind the `serde` feature.
3. Version the baked payload together with the source mask checksum and config.
4. Load the baked data directly at runtime and skip contour extraction entirely.

This avoids per-launch triangulation, simplification, and decomposition costs for static assets.

Dirty-region ECS updates are intentionally conservative:

- isolated edits with empty crop borders are merged back into the previous full output
- edits whose expanded crop still touches filled pixels on the crop boundary fall back to a full rebuild

That policy keeps the runtime correct for large continuous terrain while still allowing true partial regeneration for isolated islands, frames, or editor-authored subregions.
`ColliderGenOutput.generation` and `ColliderGenFinished.generation` expose which path was taken so
games and tools can log or react to merge vs fallback behavior explicitly.

## Composite Authoring Helpers

`BinaryImage::grow` / `BinaryImage::shrink` are authoring-friendly aliases over morphology when you want collision slightly looser or tighter than the art.

`BinaryImage::stamp_mask` / `BinaryImage::carve_mask` support tilemap and editor workflows:

- slice tile masks from an atlas
- stamp them into a larger `BinaryImage`
- run one full `generate_collider_geometry` pass to remove internal seams, similar in spirit to a composite collider workflow

## Notes

- `BinaryImage` uses top-left pixel coordinates for authored raster operations.
- Generated geometry is centered in local space by default, with `scale.x` and `scale.y` applied independently.
- Aggressive simplification is validated; invalid reductions fail or retry with a safer epsilon instead of silently returning broken polygons.

More detail lives in [architecture.md](docs/architecture.md), [configuration.md](docs/configuration.md), and [baking.md](docs/baking.md).
