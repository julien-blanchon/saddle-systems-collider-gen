use std::time::{Duration, Instant};

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    AtlasSlicer, BinaryImage, ColliderGenConfig, ColliderGenDirty, ColliderGenLod,
    ColliderGenPlugin, ColliderGenSource, ColliderGenSourceKind, Contour, simplify_contour,
};

fn main() {
    benchmark_pipeline_masks();
    benchmark_simplification();
    benchmark_atlas();
    benchmark_dirty_region();
}

fn benchmark_pipeline_masks() {
    println!("== full pipeline ==");
    for size in [256u32, 512, 2048] {
        for (label, mask) in [("sparse", sparse_mask(size)), ("dense", dense_mask(size))] {
            let started = Instant::now();
            let result = saddle_systems_collider_gen::generate_collider_geometry(
                &mask,
                &ColliderGenConfig::default().with_lod(ColliderGenLod::Medium),
            )
            .expect("pipeline mask should generate");
            println!(
                "{label:>6} {:>4}x{:<4}: {:>8.2?}  contours={:<3} pieces={:<4}",
                size,
                size,
                started.elapsed(),
                result.contours.len(),
                result.convex_pieces.len()
            );
        }
    }
}

fn benchmark_simplification() {
    println!("\n== simplification ==");
    let config = ColliderGenConfig::default().with_lod(ColliderGenLod::Low);
    for vertex_count in [100usize, 1_000, 10_000] {
        let contour = Contour::local(sawtooth_ring(vertex_count));
        let started = Instant::now();
        let (simplified, stats, _) =
            simplify_contour(&contour, &config).expect("ring simplification should succeed");
        println!(
            "ring {:>5} verts: {:>8.2?}  simplified={:<5} deviation={:.3}",
            vertex_count,
            started.elapsed(),
            simplified.points.len(),
            stats.max_deviation
        );
    }
}

fn benchmark_atlas() {
    println!("\n== atlas slicing ==");
    for tiles_per_side in [4u32, 8, 16] {
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
        println!(
            "{:>3} tiles ({:>3}x{:>2}): {:>8.2?}  filled_tiles={filled_tiles}",
            slicer.len(),
            tiles_per_side,
            tiles_per_side,
            started.elapsed()
        );
    }
}

fn benchmark_dirty_region() {
    println!("\n== dirty-region regeneration ==");
    let config = ColliderGenConfig {
        scale: Vec2::splat(1.0),
        ..ColliderGenConfig::default().with_lod(ColliderGenLod::Medium)
    };

    let original_mask = dirty_region_mask();
    let edited_mask = {
        let mut mask = original_mask.clone();
        mask.carve_circle(IVec2::new(1600, 1600), 24);
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
            region: Some(IRect::new(1568, 1568, 1632, 1632)),
        });
    }

    let dirty_started = Instant::now();
    app.update();
    let dirty_elapsed = dirty_started.elapsed();
    let dirty_output = app
        .world()
        .get::<saddle_systems_collider_gen::ColliderGenOutput>(entity)
        .expect("dirty-region output should exist");

    let full_started = Instant::now();
    let full_result = saddle_systems_collider_gen::generate_collider_geometry(&edited_mask, &config)
        .expect("full regeneration should succeed");
    let full_elapsed = full_started.elapsed();

    println!(
        "isolated edit: dirty={:>8.2?}  full={:>8.2?}  contours={} pieces={}",
        dirty_elapsed,
        full_elapsed,
        dirty_output.result.contours.len(),
        dirty_output.result.convex_pieces.len()
    );
    assert_eq!(dirty_output.result, full_result);
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

fn dirty_region_mask() -> BinaryImage {
    let mut mask = BinaryImage::new(2048, 2048);
    mask.fill_rect(0, 0, 2048, 224);
    mask.fill_rect(192, 224, 512, 160);
    mask.fill_rect(1472, 1472, 256, 192);
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

#[allow(dead_code)]
fn _format_duration(duration: Duration) -> String {
    format!("{duration:.2?}")
}
