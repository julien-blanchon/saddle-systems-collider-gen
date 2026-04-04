# Configuration

`ColliderGenConfig` is the main tuning surface.

## `ColliderGenConfig`

| Field | Type | Default | Valid Range | Effect | Common Use |
| --- | --- | --- | --- | --- | --- |
| `image` | `ImageMaskConfig` | see below | n/a | How source pixels become a binary mask | authored PNG masks, Bevy images |
| `contour_mode` | `ContourMode` | `PixelExact` | `PixelExact` or `MarchingSquares` | Chooses extraction style | exact terrain vs smoother previews |
| `scale` | `Vec2` | `Vec2::ONE` | non-zero positive values recommended | Converts pixel units to local world units independently on X and Y | non-uniform sprite scaling, stretched terrain |
| `simplification` | `SimplificationConfig` | see below | non-negative | Vertex reduction and retry behavior | LOD tuning |
| `minimum_area` | `f32` | `0.5` | `>= 0` | Drops tiny contours after extraction or reduction | ignore pixel dust and tiny anti-aliased islands |
| `minimum_vertices` | `usize` | `3` | `>= 3` | Minimum valid polygon size | guard degenerate output |
| `dirty_region_margin` | `u32` | `2` | `>= 0` | Expands dirty crops so nearby contours remain continuous | destructible terrain, editor painting |
| `lod` | `ColliderGenLod` | `High` | `High`, `Medium`, `Low` | Named fidelity preset for simplification | desktop vs mobile vs preview |
| `decomposition` | `DecompositionConfig` | see below | n/a | Enables and limits convex-piece generation | dynamic bodies, compound colliders |

## `ImageMaskConfig`

| Field | Type | Default | Valid Range | Effect | Common Use |
| --- | --- | --- | --- | --- | --- |
| `alpha_threshold` | `u8` | `128` | `0..=255` | Filled when alpha is at or above the threshold in `Alpha` mode | dedicated alpha masks |
| `brightness_threshold` | `u8` | `128` | `0..=255` | Filled when the selected color channel metric crosses the threshold | grayscale masks |
| `channel_mode` | `MaskChannelMode` | `Alpha` | enum | Which source channel drives occupancy | alpha, luma, per-channel authored masks |
| `invert_mask` | `bool` | `false` | `true` / `false` | Flips filled vs empty after thresholding | hole masks, inverted occupancy maps |
| `color_key` | `Option<ColorKey>` | `None` | optional | Treats matching keyed colors as empty before thresholding | magenta keyed collision masks |

## `MaskChannelMode`

| Variant | Meaning |
| --- | --- |
| `Alpha` | Uses source alpha |
| `Brightness` | Uses max of RGB channels |
| `Luma` | Uses weighted RGB luminance |
| `Red` / `Green` / `Blue` | Uses a single authored channel |

## `SimplificationConfig`

| Field | Type | Default | Valid Range | Effect | Common Use |
| --- | --- | --- | --- | --- | --- |
| `collinear_epsilon` | `f32` | `1e-5` | `>= 0` | Removes nearly straight runs first | all workloads |
| `rdp_epsilon` | `f32` | `0.0` | `>= 0` | Ramer-Douglas-Peucker reduction strength | terrain LOD |
| `visvalingam_area_threshold` | `f32` | `0.0` | `>= 0` | Visvalingam-Whyatt effective-area reduction strength | softer authored reductions |
| `retry_scale` | `f32` | `0.5` | `(0, 1]` recommended | Shrinks simplification epsilons after an invalid result | topology preservation |
| `max_retries` | `u32` | `3` | `>= 0` | Maximum retry attempts before failing | aggressive authoring presets |

## `DecompositionConfig`

| Field | Type | Default | Valid Range | Effect | Common Use |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true` / `false` | Enables convex-piece output generation | dynamic collision workflows |
| `max_piece_count` | `usize` | `256` | `>= 1` | Caps the number of returned pieces | broad-phase cost control |
| `min_piece_area` | `f32` | `0.25` | `>= 0` | Drops tiny convex pieces | avoid noise fragments |

## LOD Presets

`ColliderGenConfig::with_lod(...)` rewrites simplification defaults:

| LOD | Intent | Simplification profile |
| --- | --- | --- |
| `High` | authored gameplay fidelity | only collinear cleanup |
| `Medium` | balanced preview / shipping terrain | modest RDP + Visvalingam |
| `Low` | mobile / broad terrain simplification | more aggressive reduction with more retries |

Because convex decomposition runs on the final simplified contours, these LOD presets affect both `result.contours` and `result.convex_pieces`.

## Dirty Regions

Dirty-region helpers operate in source pixel space:

- `ColliderGenDirty.region`: optional `IRect`
- `dirty_region_margin`: expansion applied around that rect

Use this when:

- painting into a terrain mask
- carving explosions
- editing collision masks in an in-game tool

Keep the margin big enough to preserve contours that cross the boundary of the edited patch.

Current ECS merge rule:

- if the expanded crop has an empty border, the crate merges the regenerated region back into the previous full output
- if filled pixels still touch the expanded crop border, the crate falls back to full regeneration

This keeps dirty updates deterministic and correct without pretending that every large connected terrain edit can be stitched from a local crop alone.

## Authoring Helpers

`BinaryImage` also exposes a few workflow helpers that deliberately sit outside `ColliderGenConfig`:

- `grow(radius)` / `shrink(radius)`:
  authoring-friendly aliases over `dilate` / `erode` when collision should be looser or tighter than the art
- `stamp_mask(source, top_left)`:
  merges filled pixels from a sliced tile or authored patch into a larger mask without clearing unrelated pixels
- `carve_mask(source, top_left)`:
  removes pixels using another mask silhouette, useful for subtractive editors or destructible decals

These helpers are especially useful for tilemap composite workflows: stamp many tile masks into one larger `BinaryImage`, then extract contours once from the merged canvas to remove internal seams.

## Example Panels

The crate-local examples and lab ship with `saddle-pane` controls layered on top of the public API.
Those panels do not add new crate runtime types, but they directly drive the existing configuration
surface:

- render scale -> `ColliderGenConfig::scale`
- contour mode toggle -> `ColliderGenConfig::contour_mode`
- decomposition toggle -> `DecompositionConfig::enabled`
- simplification sliders -> `SimplificationConfig::rdp_epsilon` and `visvalingam_area_threshold`
- minimum-area slider -> `ColliderGenConfig::minimum_area`

Example-specific panel fields then feed the authored source data for that scene:

- morphology radius in `masks`
- frame cadence in `animation_frames`
- stamped tile composition in `tilemap_merge`
- blast radius in `destructible`
- lab view selection in `examples/lab`
