use bevy::{
    ecs::message::MessageCursor,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

use saddle_systems_saddle_systems_collider_gen::{
    BinaryImage, ColliderGenConfig, ColliderGenDirty, ColliderGenLod, ColliderGenPlugin,
    ColliderGenSource, ColliderGenSourceKind, ColliderGenSystems, ContourMode,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AfterGeneration;

#[test]
fn plugin_builds_with_public_system_set_ordering() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .add_plugins(ColliderGenPlugin)
        .configure_sets(Update, ColliderGenSystems::Generate.before(AfterGeneration));

    app.finish();
}

#[test]
fn ecs_regenerates_only_when_marked_dirty_or_changed() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .add_plugins(ColliderGenPlugin);

    let mut mask = BinaryImage::new(4, 4);
    mask.fill_rect(0, 0, 4, 4);
    let entity = app
        .world_mut()
        .spawn(ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(mask),
            config: default(),
        })
        .id();

    app.update();

    let mut cursor = MessageCursor::<saddle_systems_collider_gen::ColliderGenFinished>::default();
    let finished_first: Vec<_> = cursor
        .read(
            app.world()
                .resource::<Messages<saddle_systems_collider_gen::ColliderGenFinished>>(),
        )
        .cloned()
        .collect();
    assert_eq!(finished_first.len(), 1);

    app.update();
    let finished_second: Vec<_> = cursor
        .read(
            app.world()
                .resource::<Messages<saddle_systems_collider_gen::ColliderGenFinished>>(),
        )
        .cloned()
        .collect();
    assert!(finished_second.is_empty());

    app.world_mut().entity_mut(entity).insert(ColliderGenDirty {
        region: Some(IRect::new(1, 1, 3, 3)),
    });
    app.update();

    let finished_third: Vec<_> = cursor
        .read(
            app.world()
                .resource::<Messages<saddle_systems_collider_gen::ColliderGenFinished>>(),
        )
        .cloned()
        .collect();
    assert_eq!(finished_third.len(), 1);
}

#[test]
fn bevy_image_sources_generate_geometry_from_real_image_bytes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .add_plugins(ColliderGenPlugin);

    let mut image = Image::new_fill(
        Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    image
        .set_color_at(1, 1, Color::srgba(1.0, 1.0, 1.0, 0.0))
        .expect("pixel write should succeed");

    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let entity = app
        .world_mut()
        .spawn(ColliderGenSource {
            kind: ColliderGenSourceKind::Image {
                handle,
                region: None,
            },
            config: default(),
        })
        .id();

    app.update();

    let output = app
        .world()
        .get::<saddle_systems_collider_gen::ColliderGenOutput>(entity)
        .expect("output should exist after update");
    assert!(!output.result.contours.is_empty());
}

#[test]
fn isolated_dirty_regions_merge_back_into_full_output() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .add_plugins(ColliderGenPlugin);

    let mut mask = BinaryImage::new(20, 20);
    mask.fill_rect(2, 2, 3, 3);
    mask.fill_rect(8, 8, 4, 4);
    let mut expected_mask = mask.clone();

    let entity = app
        .world_mut()
        .spawn(ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(mask),
            config: default(),
        })
        .id();

    app.update();

    expected_mask.carve_rect(9, 9, 2, 2);
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        let mut source = entity_mut
            .get_mut::<ColliderGenSource>()
            .expect("source should exist");
        let ColliderGenSourceKind::Binary(mask) = &mut source.kind else {
            panic!("expected binary source");
        };
        mask.carve_rect(9, 9, 2, 2);
        entity_mut.insert(ColliderGenDirty {
            region: Some(IRect::new(8, 8, 12, 12)),
        });
    }

    app.update();

    let output = app
        .world()
        .get::<saddle_systems_collider_gen::ColliderGenOutput>(entity)
        .expect("dirty update should regenerate output");
    let expected =
        saddle_systems_collider_gen::generate_collider_geometry(&expected_mask, &default())
            .expect("full regeneration should succeed");

    assert_eq!(output.source_region, Some(URect::new(6, 6, 14, 14)));
    assert_eq!(output.result, expected);
}

#[test]
fn boundary_touching_dirty_regions_fall_back_to_full_regeneration() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Image>>()
        .add_plugins(ColliderGenPlugin);

    let mut mask = BinaryImage::new(20, 20);
    mask.fill_rect(0, 0, 20, 6);
    let mut expected_mask = mask.clone();

    let entity = app
        .world_mut()
        .spawn(ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(mask),
            config: default(),
        })
        .id();

    app.update();

    expected_mask.carve_circle(IVec2::new(10, 3), 2);
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        let mut source = entity_mut
            .get_mut::<ColliderGenSource>()
            .expect("source should exist");
        let ColliderGenSourceKind::Binary(mask) = &mut source.kind else {
            panic!("expected binary source");
        };
        mask.carve_circle(IVec2::new(10, 3), 2);
        entity_mut.insert(ColliderGenDirty {
            region: Some(IRect::new(7, 0, 13, 6)),
        });
    }

    app.update();

    let output = app
        .world()
        .get::<saddle_systems_collider_gen::ColliderGenOutput>(entity)
        .expect("dirty update should regenerate output");
    let expected =
        saddle_systems_collider_gen::generate_collider_geometry(&expected_mask, &default())
            .expect("full regeneration should succeed");

    assert_eq!(output.source_region, Some(URect::new(5, 0, 15, 8)));
    assert_eq!(output.result, expected);
}

#[test]
fn config_mutation_changes_output_deterministically() {
    let mut mask = BinaryImage::new(24, 16);
    mask.fill_rect(0, 0, 24, 4);
    mask.fill_rect(4, 7, 6, 2);
    mask.fill_polygon(&[
        Vec2::new(11.0, 4.0),
        Vec2::new(21.0, 4.0),
        Vec2::new(16.0, 13.0),
    ]);

    let high = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig::default().with_lod(ColliderGenLod::High),
    )
    .expect("high LOD generation should succeed");
    let low_first = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig::default().with_lod(ColliderGenLod::Low),
    )
    .expect("low LOD generation should succeed");
    let low_second = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig::default().with_lod(ColliderGenLod::Low),
    )
    .expect("repeat low LOD generation should succeed");

    let high_vertices: usize = high
        .contours
        .iter()
        .map(|contour| contour.points.len())
        .sum();
    let low_vertices: usize = low_first
        .contours
        .iter()
        .map(|contour| contour.points.len())
        .sum();
    let high_piece_vertices: usize = high
        .convex_pieces
        .iter()
        .map(|piece| piece.points.len())
        .sum();
    let low_piece_vertices: usize = low_first
        .convex_pieces
        .iter()
        .map(|piece| piece.points.len())
        .sum();

    assert_eq!(low_first, low_second);
    assert!(low_vertices <= high_vertices);
    assert!(low_piece_vertices <= high_piece_vertices);
}

#[test]
fn convex_pieces_follow_selected_contour_mode() {
    let mut mask = BinaryImage::new(6, 6);
    mask.fill_polygon(&[
        Vec2::new(1.0, 1.0),
        Vec2::new(5.0, 1.0),
        Vec2::new(3.0, 5.0),
    ]);

    let pixel_exact = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig::default().with_lod(ColliderGenLod::High),
    )
    .expect("pixel exact generation should succeed");
    let marching = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig {
            contour_mode: ContourMode::MarchingSquares,
            ..ColliderGenConfig::default().with_lod(ColliderGenLod::High)
        },
    )
    .expect("marching squares generation should succeed");

    assert_ne!(pixel_exact.contours, marching.contours);
    assert_ne!(pixel_exact.convex_pieces, marching.convex_pieces);
}
