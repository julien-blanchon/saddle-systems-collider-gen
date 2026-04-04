use bevy::platform::time::Instant;
use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_systems_collider_gen::{
    AtlasSlicer, BinaryImage, ColliderGenConfig, ColliderGenDirty, ColliderGenLod,
    ColliderGenOutput, ColliderGenPlugin, ColliderGenSource, ColliderGenSourceKind, Contour,
    simplify_contour,
};
use saddle_systems_collider_gen_example_support as support;

#[derive(Resource, Pane)]
#[pane(title = "Perf Bench", position = "bottom-right")]
struct PerfBenchPane {
    #[pane(slider, min = 128.0, max = 1024.0, step = 128.0)]
    mask_size: u32,
    #[pane(slider, min = 100.0, max = 10000.0, step = 100.0)]
    simplification_vertices: u32,
    #[pane(slider, min = 4.0, max = 16.0, step = 4.0)]
    atlas_tiles_per_side: u32,
    #[pane(slider, min = 512.0, max = 2048.0, step = 512.0)]
    dirty_region_size: u32,
}

impl Default for PerfBenchPane {
    fn default() -> Self {
        Self {
            mask_size: 512,
            simplification_vertices: 2_400,
            atlas_tiles_per_side: 8,
            dirty_region_size: 2048,
        }
    }
}

#[derive(Resource, Default)]
struct PerfSummary {
    full_pipeline: String,
    simplification: String,
    atlas: String,
    dirty_region: String,
}

#[derive(Component)]
struct PerfOverlay;

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen perf");
    app.init_resource::<PerfBenchPane>()
        .init_resource::<PerfSummary>()
        .register_pane::<PerfBenchPane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (run_benchmarks, update_overlay).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Perf Overlay"),
        PerfOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            top: Val::Px(18.0),
            width: Val::Px(520.0),
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.86)),
        Text::new("collider_gen perf"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn run_benchmarks(
    pane: Res<PerfBenchPane>,
    mut summary: ResMut<PerfSummary>,
) {
    if !pane.is_changed() && !summary.full_pipeline.is_empty() {
        return;
    }

    summary.full_pipeline = benchmark_pipeline_masks(pane.mask_size);
    summary.simplification = benchmark_simplification(pane.simplification_vertices as usize);
    summary.atlas = benchmark_atlas(pane.atlas_tiles_per_side);
    summary.dirty_region = benchmark_dirty_region(pane.dirty_region_size);
}

fn update_overlay(
    summary: Res<PerfSummary>,
    mut overlay: Single<&mut Text, With<PerfOverlay>>,
) {
    if !summary.is_changed() {
        return;
    }

    **overlay = Text::new(format!(
        "collider_gen perf\n{}\n{}\n{}\n{}",
        summary.full_pipeline, summary.simplification, summary.atlas, summary.dirty_region
    ));
}

fn benchmark_pipeline_masks(size: u32) -> String {
    let sparse = sparse_mask(size);
    let dense = dense_mask(size);
    let config = ColliderGenConfig::default().with_lod(ColliderGenLod::Medium);

    let started = Instant::now();
    let sparse_result =
        saddle_systems_collider_gen::generate_collider_geometry(&sparse, &config)
            .expect("sparse mask should generate");
    let sparse_ms = started.elapsed().as_secs_f32() * 1_000.0;

    let started = Instant::now();
    let dense_result =
        saddle_systems_collider_gen::generate_collider_geometry(&dense, &config)
            .expect("dense mask should generate");
    let dense_ms = started.elapsed().as_secs_f32() * 1_000.0;

    format!(
        "full pipeline {size}^2: sparse {:.2}ms ({} contours / {} pieces), dense {:.2}ms ({} contours / {} pieces)",
        sparse_ms,
        sparse_result.contours.len(),
        sparse_result.convex_pieces.len(),
        dense_ms,
        dense_result.contours.len(),
        dense_result.convex_pieces.len(),
    )
}

fn benchmark_simplification(vertex_count: usize) -> String {
    let config = ColliderGenConfig::default().with_lod(ColliderGenLod::Low);
    let contour = Contour::local(sawtooth_ring(vertex_count));
    let started = Instant::now();
    let (simplified, stats, _) =
        simplify_contour(&contour, &config).expect("ring simplification should succeed");
    let elapsed_ms = started.elapsed().as_secs_f32() * 1_000.0;
    format!(
        "simplification {} verts: {:.2}ms -> {} verts (max dev {:.3})",
        vertex_count,
        elapsed_ms,
        simplified.points.len(),
        stats.max_deviation
    )
}

fn benchmark_atlas(tiles_per_side: u32) -> String {
    let atlas_mask = atlas_mask(tiles_per_side);
    let slicer = AtlasSlicer::from_grid(
        atlas_mask,
        UVec2::new(16, 16),
        tiles_per_side,
        tiles_per_side,
        None,
        None,
    );
    let started = Instant::now();
    let filled_tiles = (0..slicer.len())
        .filter_map(|index| slicer.slice_index(index).ok())
        .filter(|tile| tile.filled_count() > 0)
        .count();
    let elapsed_ms = started.elapsed().as_secs_f32() * 1_000.0;
    format!(
        "atlas {}x{}: {:.2}ms to inspect {} tiles ({} filled)",
        tiles_per_side,
        tiles_per_side,
        elapsed_ms,
        slicer.len(),
        filled_tiles
    )
}

fn benchmark_dirty_region(size: u32) -> String {
    let config = ColliderGenConfig {
        scale: Vec2::splat(1.0),
        ..ColliderGenConfig::default().with_lod(ColliderGenLod::Medium)
    };

    let original_mask = dirty_region_mask(size);
    let edited_mask = {
        let mut mask = original_mask.clone();
        let center = (size.saturating_sub(1) / 2) as i32;
        let radius = (size / 80).max(12) as i32;
        mask.carve_circle(IVec2::new(center, center), radius);
        mask
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .add_plugins(ColliderGenPlugin);

    let entity = app
        .world_mut()
        .spawn(ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(original_mask),
            config,
        })
        .id();
    app.update();

    let edit_center = (size.saturating_sub(1) / 2) as i32;
    let edit_radius = (size / 80).max(12) as i32;
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        let mut source = entity_mut
            .get_mut::<ColliderGenSource>()
            .expect("perf source should exist");
        let ColliderGenSourceKind::Binary(mask) = &mut source.kind else {
            panic!("expected binary source");
        };
        *mask = edited_mask.clone();
        entity_mut.insert(ColliderGenDirty {
            region: Some(IRect::new(
                edit_center - edit_radius - 2,
                edit_center - edit_radius - 2,
                edit_center + edit_radius + 2,
                edit_center + edit_radius + 2,
            )),
        });
    }

    let dirty_started = Instant::now();
    app.update();
    let dirty_elapsed = dirty_started.elapsed().as_secs_f32() * 1_000.0;
    let dirty_output = app
        .world()
        .get::<ColliderGenOutput>(entity)
        .expect("dirty-region output should exist");

    let full_started = Instant::now();
    let full_result =
        saddle_systems_collider_gen::generate_collider_geometry(&edited_mask, &config)
            .expect("full regeneration should succeed");
    let full_elapsed = full_started.elapsed().as_secs_f32() * 1_000.0;

    assert_eq!(dirty_output.result, full_result);
    format!(
        "dirty regen {}^2: dirty {:.2}ms vs full {:.2}ms ({} contours / {} pieces)",
        size,
        dirty_elapsed,
        full_elapsed,
        dirty_output.result.contours.len(),
        dirty_output.result.convex_pieces.len(),
    )
}

fn sparse_mask(size: u32) -> BinaryImage {
    let mut mask = BinaryImage::new(size, size);
    let island = (size / 10).max(8);
    mask.fill_rect(size / 6, size / 6, island, island);
    mask.fill_rect(size / 2, size / 2, island + island / 2, island);
    mask.fill_circle(
        IVec2::new((size * 3 / 4) as i32, (size * 2 / 5) as i32),
        (size / 12) as i32,
    );
    mask
}

fn dense_mask(size: u32) -> BinaryImage {
    let mut mask = BinaryImage::new(size, size);
    let floor = (size / 7).max(16);
    mask.fill_rect(0, 0, size, floor);
    mask.fill_rect(size / 8, floor, size * 3 / 4, size / 2);
    mask.carve_circle(
        IVec2::new((size / 2) as i32, (size / 3) as i32),
        (size / 10) as i32,
    );
    mask.carve_rect(size / 5, floor + size / 8, size / 8, size / 10);
    mask
}

fn atlas_mask(tiles_per_side: u32) -> BinaryImage {
    let tile_size = 16u32;
    let size = tiles_per_side * tile_size;
    let mut mask = BinaryImage::new(size, size);

    for row in 0..tiles_per_side {
        for column in 0..tiles_per_side {
            if (row + column) % 3 == 0 {
                let offset = UVec2::new(column * tile_size, row * tile_size);
                mask.fill_rect(offset.x + 2, offset.y + 2, tile_size - 4, tile_size - 4);
            }
        }
    }

    mask
}

fn dirty_region_mask(size: u32) -> BinaryImage {
    let mut mask = BinaryImage::new(size, size);
    let floor = (size / 10).max(64);
    mask.fill_rect(0, 0, size, floor);
    mask.fill_rect(size / 10, floor, size / 4, size / 12);
    mask.fill_rect(size * 7 / 10, size * 7 / 10, size / 8, size / 10);
    mask
}

fn sawtooth_ring(vertex_count: usize) -> Vec<Vec2> {
    let radius = 120.0f32;
    (0..vertex_count)
        .map(|index| {
            let angle = index as f32 / vertex_count as f32 * std::f32::consts::TAU;
            let pulse = if index % 2 == 0 { 0.0 } else { 10.0 };
            let distance = radius + pulse;
            Vec2::new(angle.cos() * distance, angle.sin() * distance)
        })
        .collect()
}
