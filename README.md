# Saddle Systems Collider Gen

Engine-agnostic 2D collision geometry generation for Bevy. The crate turns binary masks, Bevy `Image` assets, atlas subregions, and procedural raster edits into reusable outline loops, topology metadata, convex hulls, and convex-piece compounds without depending on any physics backend.

## Quick Start

```toml
[dependencies]
saddle-systems-collider-gen = { workspace = true }
```

```rust
use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, ColliderGenConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut mask = BinaryImage::new(24, 16);
mask.fill_rect(0, 0, 24, 3);
mask.fill_rect(4, 6, 8, 2);
mask.carve_circle(IVec2::new(18, 2), 2);

let result = saddle_systems_collider_gen::generate_collider_geometry(&mask, &ColliderGenConfig::default())?;

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
| `CoordinateTransform` | Pixel-corner to local-centered coordinate conversion with non-uniform scale |

### ECS integration

| Type | Purpose |
| --- | --- |
| `ColliderGenPlugin` | Registers ECS systems, messages, and reflected component types |
| `ColliderGenSystems` | Public ordering hooks: `Extract`, `Generate`, `Validate`, `Cache` |
| `ColliderGenSource` | ECS source component holding a binary mask or Bevy image handle plus config; intentionally not `serde`-serializable because `Handle<Image>` is runtime Bevy state |
| `ColliderGenDirty` | Optional dirty-region marker for targeted regeneration; isolated edits merge back into the previous full result, while boundary-touching crops fall back to full regeneration for correctness |
| `ColliderGenOutput` | Generated geometry plus cached summary metadata |
| `ColliderGenFinished` / `ColliderGenFailed` | Buffered completion messages |

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

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal authored-mask workflow with outlines, hulls, and convex pieces | `cargo run -p saddle-systems-collider-gen-example-basic` |
| `atlas` | Atlas slicing and per-frame / per-tile generation using the `image` feature | `cargo run -p saddle-systems-collider-gen-example-atlas --features image` |
| `masks` | Binary-image morphology and cleanup workflows | `cargo run -p saddle-systems-collider-gen-example-masks` |
| `destructible` | ECS-driven dirty regeneration on a mutable terrain mask | `cargo run -p saddle-systems-collider-gen-example-destructible` |
| `animation_frames` | Precomputed frame geometry from a spritesheet-like atlas | `cargo run -p saddle-systems-collider-gen-example-animation-frames` |
| `debug_gizmos` | Pixel-exact vs marching-squares comparison with BRP extras enabled | `cargo run -p saddle-systems-collider-gen-example-debug-gizmos` |
| `perf` | Release/debug-oriented timings for sparse+dense masks, simplification, atlas slicing, and dirty-region regen | `cargo run -p saddle-systems-collider-gen-example-perf` |

## Workspace Lab

This repository also provides a crate-local lab app at
`shared/systems/saddle-systems-collider-gen/examples/lab`:

```bash
cargo run -p saddle-systems-collider-gen-lab
```

Crate-local E2E scenarios live with that lab instead of `crates/e2e`:

```bash
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_smoke
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_thresholds
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_atlas
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_composite
cargo run -p saddle-systems-collider-gen-lab --features e2e -- collider_gen_destructible
```

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
