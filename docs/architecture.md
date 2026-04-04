# Architecture

`saddle-systems-collider-gen` is split into two layers:

1. Pure raster and geometry code
2. Thin ECS integration for Bevy apps that want asset-driven or dirty-region workflows

## Pipeline

```text
source bytes / Bevy Image / BinaryImage
    -> threshold extraction
    -> optional crop / atlas slice / dirty-region crop
       -> atlas batches can be pre-baked with `bake_atlas_collider_frames(...)`
    -> optional mask authoring helpers
       -> grow / shrink via morphology
       -> stamp / carve sub-masks for tilemap-style composition
    -> dirty-region merge back into previous full result
       -> or correctness-first full fallback if the crop still touches filled boundary pixels
    -> contour extraction
       -> pixel-exact boundary tracing
       -> or marching squares
    -> topology build
    -> simplification + validation
    -> optional hull / triangulation / convex decomposition
       -> decomposition runs on the same simplified contour set that will be published
    -> ColliderGenResult
```

The important design choice is that the geometry stages do not know about physics engines. The output is reusable by any downstream consumer.

The ECS layer keeps dirty updates conservative instead of pretending every crop can be stitched safely. If the expanded dirty crop still contains filled pixels on its outer border, the runtime regenerates from the full mask because the edited shape is still connected to geometry outside the crop.

## Contour Modes

### PixelExact

- Built from boundary edges around filled pixels
- Preserves authored collision intent precisely
- Best for platform terrain, pixel art, and baked static masks
- Produces axis-aligned corner points in local-centered space

### MarchingSquares

- Built from a padded sample field around the mask
- Places points on half-pixel boundaries for a smoother silhouette
- Best for softer terrain previews and rounded procedural data
- More visually smooth, but less literal than `PixelExact`

## Topology Model

Each contour stores only its loop. Hole and nesting information lives in `ContourTopology`:

- `parent = None`: top-level island
- `parent = Some(i)`: contained by contour `i`
- `children`: nested contours directly inside this loop
- `is_hole`: derived from winding and containment

Documented winding convention:

- outer contours: counter-clockwise
- holes: clockwise

The crate normalizes winding after simplification so downstream consumers get a stable convention even when they change LOD or simplification settings.
Topology is rebuilt from the simplified contours before the final result is published, so `ContourTopology`, `contours`, and `convex_pieces` stay in sync.

## Simplification Failure Modes

Simplification is intentionally defensive.

Potential failure modes:

- duplicate vertices
- zero-length edges
- zero-area output
- self-intersections introduced by reduction
- winding flips that invert hole meaning

Response strategy:

1. remove duplicates and degenerate edges first
2. remove collinear runs
3. apply RDP and Visvalingam if enabled
4. validate
5. retry with smaller epsilons when configured
6. return an explicit error if the polygon still becomes invalid

The crate does not silently ship invalid simplified geometry.

## Convex Decomposition Notes

The initial decomposition path is intentionally conservative:

- simple polygons are triangulated with ear clipping
- adjacent triangles are merged only if the merged ring remains convex and simple

This is reliable for simple outer contours and low piece counts. For hole-heavy regions, `contours + topology` is currently the more faithful source of truth. Hole-aware bridge construction is the main follow-up improvement after this pass.

## Performance Notes

Relative cost by stage:

- threshold extraction: cheap
- atlas slicing / cropping: cheap
- pixel-exact contour extraction: moderate
- marching squares: moderate
- simplification validation: moderate to expensive on large vertex counts
- triangulation / convex decomposition: most expensive stage

Practical guidance:

- use runtime generation for tools, previews, and rare edits
- use `stamp_mask` to compose many per-tile masks into one larger canvas before extracting final runtime geometry
- use the `tilemap_merge` example as the reference pattern when you want one collider result from many reusable room, ground, or wall tiles
- pre-bake authored content
- keep destructible dirty regions small
- expect boundary-touching dirty edits on large continuous terrain to fall back to full regeneration
- use lower LODs for large terrain where perfect per-pixel collision is unnecessary
