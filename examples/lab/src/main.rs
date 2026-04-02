#[cfg(feature = "e2e")]
mod e2e;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use saddle_systems_collider_gen::{
    AtlasSlicer, BinaryImage, ColliderGenConfig, ColliderGenDirty, ColliderGenFinished,
    ColliderGenOutput, ColliderGenPlugin, ColliderGenSource, ColliderGenSourceKind,
    ColliderGenSystems, ColliderGenWarning, Contour, ContourMode, CoordinateTransform,
    extract_pixel_exact_contours,
};

#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;

const LAB_SCALE: f32 = 20.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
enum LabView {
    #[default]
    Overview,
    Thresholds,
    Atlas,
    Composite,
    Destructible,
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
struct ActiveView {
    current: LabView,
}

impl Default for ActiveView {
    fn default() -> Self {
        Self {
            current: LabView::Overview,
        }
    }
}

#[derive(Debug, Clone, Default, Reflect)]
struct OverviewDiagnostics {
    raw_contours: usize,
    simplified_contours: usize,
    marching_contours: usize,
    convex_pieces: usize,
}

#[derive(Debug, Clone, Default, Reflect)]
struct ThresholdDiagnostics {
    alpha_filled: usize,
    luma_filled: usize,
    keyed_filled: usize,
}

#[derive(Debug, Clone, Default, Reflect)]
struct AtlasDiagnostics {
    atlas_tiles: usize,
    non_empty_tiles: usize,
    total_tile_contours: usize,
}

#[derive(Debug, Clone, Default, Reflect)]
struct CompositeDiagnostics {
    placed_tiles: usize,
    per_tile_contours: usize,
    composite_contours: usize,
    composite_pieces: usize,
}

#[derive(Debug, Clone, Default, Reflect)]
struct DestructibleDiagnostics {
    blasts_applied: usize,
    regenerations: usize,
    initial_filled_pixels: usize,
    current_filled_pixels: usize,
    initial_contours: usize,
    current_contours: usize,
    current_pieces: usize,
    warnings: usize,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
struct LabDiagnostics {
    active_view: LabView,
    overview: OverviewDiagnostics,
    thresholds: ThresholdDiagnostics,
    atlas: AtlasDiagnostics,
    composite: CompositeDiagnostics,
    destructible: DestructibleDiagnostics,
}

#[derive(Resource)]
struct OverviewScene {
    mask: BinaryImage,
    transform: CoordinateTransform,
    raw_contours: Vec<Contour>,
    pixel_result: saddle_systems_collider_gen::ColliderGenResult,
    marching_result: saddle_systems_collider_gen::ColliderGenResult,
}

#[derive(Clone)]
struct ThresholdPanel {
    mask: BinaryImage,
    offset: Vec2,
}

#[derive(Resource)]
struct ThresholdScene {
    panels: Vec<ThresholdPanel>,
}

#[derive(Clone)]
struct AtlasTilePreview {
    mask: BinaryImage,
    result: saddle_systems_collider_gen::ColliderGenResult,
    offset: Vec2,
}

#[derive(Resource)]
struct AtlasScene {
    tiles: Vec<AtlasTilePreview>,
}

#[derive(Resource)]
struct CompositeScene {
    mask: BinaryImage,
    transform: CoordinateTransform,
    per_tile_contours: Vec<Vec<Vec2>>,
    composite_result: saddle_systems_collider_gen::ColliderGenResult,
}

#[derive(Resource)]
struct DestructiblePattern {
    initial_mask: BinaryImage,
    centers: Vec<IVec2>,
    index: usize,
    timer: Timer,
}

#[derive(Component)]
struct OverlayText;

#[derive(Component)]
struct LabDestructibleSource;

#[cfg(feature = "e2e")]
pub(crate) fn set_active_view(world: &mut World, view: LabView) {
    world.resource_mut::<ActiveView>().current = view;
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "collider_gen lab".to_string(),
            resolution: (1600, 960).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
    .init_resource::<ActiveView>()
    .init_resource::<LabDiagnostics>()
    .insert_resource(DestructiblePattern {
        initial_mask: build_destructible_mask(),
        centers: vec![
            IVec2::new(8, 4),
            IVec2::new(18, 5),
            IVec2::new(12, 9),
            IVec2::new(22, 4),
        ],
        index: 0,
        timer: Timer::from_seconds(0.45, TimerMode::Repeating),
    })
    .register_type::<LabView>()
    .register_type::<ActiveView>()
    .register_type::<OverviewDiagnostics>()
    .register_type::<ThresholdDiagnostics>()
    .register_type::<AtlasDiagnostics>()
    .register_type::<CompositeDiagnostics>()
    .register_type::<DestructibleDiagnostics>()
    .register_type::<LabDiagnostics>()
    .add_plugins(ColliderGenPlugin);

    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::default());

    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::ColliderGenLabE2EPlugin);

    app.add_systems(Startup, setup_lab);
    app.add_systems(
        Update,
        (
            handle_keyboard_view_switches,
            sync_active_view_diagnostics,
            reset_destructible_scene.before(ColliderGenSystems::Extract),
            apply_destructible_blasts.before(ColliderGenSystems::Extract),
            track_destructible_generations.after(ColliderGenSystems::Cache),
            draw_active_scene.after(ColliderGenSystems::Cache),
            update_overlay.after(ColliderGenSystems::Cache),
        ),
    );

    app.run();
}

fn setup_lab(mut commands: Commands, mut diagnostics: ResMut<LabDiagnostics>) {
    commands.spawn((Name::new("Collider Gen Lab Camera"), Camera2d));

    let overview = build_overview_scene();
    diagnostics.overview = OverviewDiagnostics {
        raw_contours: overview.raw_contours.len(),
        simplified_contours: overview.pixel_result.contours.len(),
        marching_contours: overview.marching_result.contours.len(),
        convex_pieces: overview.pixel_result.convex_pieces.len(),
    };
    commands.insert_resource(overview);

    let (threshold_scene, threshold_diagnostics) = build_threshold_scene();
    diagnostics.thresholds = threshold_diagnostics;
    commands.insert_resource(threshold_scene);

    let (atlas_scene, atlas_diagnostics) = build_atlas_scene();
    diagnostics.atlas = atlas_diagnostics;
    commands.insert_resource(atlas_scene);

    let (composite_scene, composite_diagnostics) = build_composite_scene();
    diagnostics.composite = composite_diagnostics;
    commands.insert_resource(composite_scene);

    let initial_mask = build_destructible_mask();
    let initial_result = saddle_systems_collider_gen::generate_collider_geometry(
        &initial_mask,
        &ColliderGenConfig {
            scale: Vec2::splat(LAB_SCALE),
            ..default()
        },
    )
    .expect("destructible seed mask should generate");
    diagnostics.destructible = DestructibleDiagnostics {
        initial_filled_pixels: initial_mask.filled_count(),
        current_filled_pixels: initial_mask.filled_count(),
        initial_contours: initial_result.contours.len(),
        current_contours: initial_result.contours.len(),
        current_pieces: initial_result.convex_pieces.len(),
        warnings: initial_result.warnings.len(),
        ..default()
    };

    commands.spawn((
        Name::new("Collider Gen Lab Destructible Source"),
        LabDestructibleSource,
        ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(initial_mask),
            config: ColliderGenConfig {
                scale: Vec2::splat(LAB_SCALE),
                ..ColliderGenConfig::default().with_lod(saddle_systems_collider_gen::ColliderGenLod::Medium)
            },
        },
    ));

    commands.spawn((
        Name::new("Collider Gen Lab Overlay"),
        OverlayText,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            width: Val::Px(520.0),
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.10, 0.82)),
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn build_overview_scene() -> OverviewScene {
    let mut mask = BinaryImage::new(30, 18);
    mask.fill_rect(0, 0, 30, 4);
    mask.fill_rect(3, 7, 8, 2);
    mask.fill_rect(18, 8, 9, 2);
    mask.fill_polygon(&[
        Vec2::new(10.0, 4.0),
        Vec2::new(20.0, 4.0),
        Vec2::new(15.0, 13.0),
    ]);
    mask.carve_circle(IVec2::new(21, 5), 2);

    let scale = Vec2::splat(LAB_SCALE);
    let transform = CoordinateTransform::centered(mask.width(), mask.height(), scale);
    let (raw_contours, _) = extract_pixel_exact_contours(
        &mask,
        CoordinateTransform::centered(mask.width(), mask.height(), scale),
    )
    .expect("raw contours should extract");
    let pixel_result = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig {
            scale,
            ..ColliderGenConfig::default().with_lod(saddle_systems_collider_gen::ColliderGenLod::Medium)
        },
    )
    .expect("pixel exact overview result should generate");
    let marching_result = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig {
            scale,
            contour_mode: ContourMode::MarchingSquares,
            ..ColliderGenConfig::default().with_lod(saddle_systems_collider_gen::ColliderGenLod::Low)
        },
    )
    .expect("marching squares overview result should generate");

    OverviewScene {
        mask,
        transform,
        raw_contours,
        pixel_result,
        marching_result,
    }
}

fn build_threshold_scene() -> (ThresholdScene, ThresholdDiagnostics) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 12,
            height: 10,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for y in 1..9 {
        for x in 1..11 {
            let alpha = if x < 4 {
                255
            } else if x < 8 {
                160
            } else {
                60
            };
            let color = if y < 4 {
                Color::srgba_u8(255, 255, 255, alpha)
            } else if y < 7 {
                Color::srgba_u8(220, 40, 40, alpha)
            } else {
                Color::srgba_u8(255, 0, 255, alpha)
            };
            image
                .set_color_at(x, y, color)
                .expect("threshold scene pixel write should succeed");
        }
    }

    let alpha_mask = BinaryImage::from_bevy_image(
        &image,
        &saddle_systems_collider_gen::ImageMaskConfig {
            channel_mode: saddle_systems_collider_gen::MaskChannelMode::Alpha,
            alpha_threshold: 120,
            ..default()
        },
    )
    .expect("alpha threshold mask should decode");
    let luma_mask = BinaryImage::from_bevy_image(
        &image,
        &saddle_systems_collider_gen::ImageMaskConfig {
            channel_mode: saddle_systems_collider_gen::MaskChannelMode::Luma,
            brightness_threshold: 110,
            ..default()
        },
    )
    .expect("luma threshold mask should decode");
    let keyed_mask = BinaryImage::from_bevy_image(
        &image,
        &saddle_systems_collider_gen::ImageMaskConfig {
            channel_mode: saddle_systems_collider_gen::MaskChannelMode::Alpha,
            alpha_threshold: 120,
            color_key: Some(saddle_systems_collider_gen::ColorKey {
                rgba: [255, 0, 255, 255],
                tolerance: 0,
            }),
            ..default()
        },
    )
    .expect("color keyed mask should decode");

    let panels = vec![
        ThresholdPanel {
            mask: alpha_mask.clone(),
            offset: Vec2::new(-420.0, 0.0),
        },
        ThresholdPanel {
            mask: luma_mask.clone(),
            offset: Vec2::ZERO,
        },
        ThresholdPanel {
            mask: keyed_mask.clone(),
            offset: Vec2::new(420.0, 0.0),
        },
    ];

    (
        ThresholdScene { panels },
        ThresholdDiagnostics {
            alpha_filled: alpha_mask.filled_count(),
            luma_filled: luma_mask.filled_count(),
            keyed_filled: keyed_mask.filled_count(),
        },
    )
}

fn build_atlas_scene() -> (AtlasScene, AtlasDiagnostics) {
    let mut atlas = BinaryImage::new(48, 32);
    atlas.fill_rect(1, 1, 12, 12);
    atlas.fill_circle(IVec2::new(24, 8), 5);
    atlas.fill_polygon(&[
        Vec2::new(35.0, 1.0),
        Vec2::new(46.0, 1.0),
        Vec2::new(40.0, 13.0),
    ]);
    atlas.fill_rect(2, 18, 10, 3);
    atlas.fill_rect(18, 18, 12, 12);
    atlas.carve_circle(IVec2::new(24, 24), 3);
    atlas.fill_rect(34, 18, 12, 4);

    let tile_scale = Vec2::splat(LAB_SCALE * 0.95);
    let slicer = AtlasSlicer::from_grid(atlas, UVec2::new(16, 16), 3, 2, None, None);
    let mut tiles = Vec::new();
    let mut total_tile_contours = 0;
    for region in slicer.iter_regions() {
        let mask = slicer
            .slice_rect(region.rect)
            .expect("atlas tile should slice successfully");
        if mask.filled_count() == 0 {
            continue;
        }

        let result = saddle_systems_collider_gen::generate_collider_geometry(
            &mask,
            &ColliderGenConfig {
                scale: tile_scale,
                ..ColliderGenConfig::default()
            },
        )
        .expect("atlas tile collider generation should succeed");
        total_tile_contours += result.contours.len();
        tiles.push(AtlasTilePreview {
            mask,
            result,
            offset: Vec2::new(
                region.column as f32 * 430.0 - 430.0,
                170.0 - region.row as f32 * 320.0,
            ),
        });
    }

    (
        AtlasScene { tiles },
        AtlasDiagnostics {
            atlas_tiles: slicer.len(),
            non_empty_tiles: slicer
                .iter_regions()
                .filter_map(|region| slicer.slice_rect(region.rect).ok())
                .filter(|mask| mask.filled_count() > 0)
                .count(),
            total_tile_contours,
        },
    )
}

fn build_composite_scene() -> (CompositeScene, CompositeDiagnostics) {
    let mut atlas = BinaryImage::new(48, 16);
    atlas.fill_rect(0, 0, 16, 16);
    atlas.fill_rect(18, 0, 10, 16);
    atlas.fill_rect(32, 0, 16, 6);

    let scale = Vec2::splat(LAB_SCALE);
    let slicer = AtlasSlicer::from_grid(
        atlas,
        UVec2::new(16, 16),
        3,
        1,
        Some(UVec2::new(0, 0)),
        None,
    );
    let placements = [
        (0usize, UVec2::new(0, 8)),
        (0usize, UVec2::new(16, 8)),
        (1usize, UVec2::new(32, 8)),
        (2usize, UVec2::new(0, 0)),
        (2usize, UVec2::new(16, 0)),
    ];

    let mut mask = BinaryImage::new(48, 24);
    let transform = CoordinateTransform::centered(mask.width(), mask.height(), scale);
    let mut per_tile_world_loops = Vec::new();
    let mut per_tile_contours = 0;

    for (index, origin) in placements {
        let tile = slicer
            .slice_index(index)
            .expect("composite tile should slice successfully");
        mask.stamp_mask(&tile, origin);

        let result = saddle_systems_collider_gen::generate_collider_geometry(
            &tile,
            &ColliderGenConfig {
                scale,
                ..ColliderGenConfig::default()
            },
        )
        .expect("per-tile generation should succeed");
        per_tile_contours += result.contours.len();
        let translation = tile_translation(
            origin,
            UVec2::new(tile.width(), tile.height()),
            UVec2::new(mask.width(), mask.height()),
            scale,
        );
        for contour in &result.contours {
            per_tile_world_loops.push(
                contour
                    .points
                    .iter()
                    .map(|point| *point + translation)
                    .collect(),
            );
        }
    }

    let composite_result = saddle_systems_collider_gen::generate_collider_geometry(
        &mask,
        &ColliderGenConfig {
            scale,
            ..ColliderGenConfig::default()
        },
    )
    .expect("composite generation should succeed");
    let diagnostics = CompositeDiagnostics {
        placed_tiles: placements.len(),
        per_tile_contours,
        composite_contours: composite_result.contours.len(),
        composite_pieces: composite_result.convex_pieces.len(),
    };

    (
        CompositeScene {
            mask,
            transform,
            per_tile_contours: per_tile_world_loops,
            composite_result,
        },
        diagnostics,
    )
}

fn build_destructible_mask() -> BinaryImage {
    let mut mask = BinaryImage::new(30, 18);
    mask.fill_rect(0, 0, 30, 5);
    mask.fill_rect(4, 8, 7, 2);
    mask.fill_rect(16, 10, 11, 2);
    mask
}

fn handle_keyboard_view_switches(
    input: Res<ButtonInput<KeyCode>>,
    mut active_view: ResMut<ActiveView>,
) {
    if input.just_pressed(KeyCode::Digit1) {
        active_view.current = LabView::Overview;
    }
    if input.just_pressed(KeyCode::Digit2) {
        active_view.current = LabView::Thresholds;
    }
    if input.just_pressed(KeyCode::Digit3) {
        active_view.current = LabView::Atlas;
    }
    if input.just_pressed(KeyCode::Digit4) {
        active_view.current = LabView::Composite;
    }
    if input.just_pressed(KeyCode::Digit5) {
        active_view.current = LabView::Destructible;
    }
}

fn sync_active_view_diagnostics(
    active_view: Res<ActiveView>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    if active_view.is_changed() {
        diagnostics.active_view = active_view.current;
    }
}

fn reset_destructible_scene(
    active_view: Res<ActiveView>,
    mut pattern: ResMut<DestructiblePattern>,
    mut diagnostics: ResMut<LabDiagnostics>,
    mut query: Query<&mut ColliderGenSource, With<LabDestructibleSource>>,
) {
    if !active_view.is_changed() || active_view.current != LabView::Destructible {
        return;
    }

    let Ok(mut source) = query.single_mut() else {
        return;
    };
    source.kind = ColliderGenSourceKind::Binary(pattern.initial_mask.clone());
    pattern.index = 0;
    pattern.timer.reset();
    diagnostics.destructible.blasts_applied = 0;
    diagnostics.destructible.regenerations = 0;
    diagnostics.destructible.current_filled_pixels = diagnostics.destructible.initial_filled_pixels;
    diagnostics.destructible.current_contours = diagnostics.destructible.initial_contours;
}

fn apply_destructible_blasts(
    active_view: Res<ActiveView>,
    time: Res<Time>,
    mut pattern: ResMut<DestructiblePattern>,
    mut diagnostics: ResMut<LabDiagnostics>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut ColliderGenSource), With<LabDestructibleSource>>,
) {
    if active_view.current != LabView::Destructible {
        return;
    }

    pattern.timer.tick(time.delta());
    if !pattern.timer.just_finished() || pattern.centers.is_empty() {
        return;
    }

    let Ok((entity, mut source)) = query.single_mut() else {
        return;
    };
    let center = pattern.centers[pattern.index % pattern.centers.len()];
    pattern.index += 1;

    if let ColliderGenSourceKind::Binary(mask) = &mut source.kind {
        mask.carve_circle(center, 2);
        commands.entity(entity).insert(ColliderGenDirty {
            region: Some(IRect::new(
                center.x - 3,
                center.y - 3,
                center.x + 3,
                center.y + 3,
            )),
        });
        diagnostics.destructible.blasts_applied += 1;
    }
}

fn track_destructible_generations(
    mut finished: MessageReader<ColliderGenFinished>,
    mut diagnostics: ResMut<LabDiagnostics>,
    query: Query<(&ColliderGenSource, &ColliderGenOutput), With<LabDestructibleSource>>,
) {
    let Ok((source, output)) = query.single() else {
        return;
    };

    for _ in finished.read() {
        diagnostics.destructible.regenerations += 1;
        if let ColliderGenSourceKind::Binary(mask) = &source.kind {
            diagnostics.destructible.current_filled_pixels = mask.filled_count();
        }
        diagnostics.destructible.current_contours = output.result.contours.len();
        diagnostics.destructible.current_pieces = output.result.convex_pieces.len();
        diagnostics.destructible.warnings = output.result.warnings.len();
    }
}

fn update_overlay(
    active_view: Res<ActiveView>,
    diagnostics: Res<LabDiagnostics>,
    mut overlay: Query<&mut Text, With<OverlayText>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };

    let body = match active_view.current {
        LabView::Overview => format!(
            "View: Overview (1)\nraw contours: {}\nsimplified contours: {}\nmarching contours: {}\nconvex pieces: {}\n\nShows authored mask, raw pixel-exact loops, simplified output, and convex pieces.",
            diagnostics.overview.raw_contours,
            diagnostics.overview.simplified_contours,
            diagnostics.overview.marching_contours,
            diagnostics.overview.convex_pieces,
        ),
        LabView::Thresholds => format!(
            "View: Thresholds (2)\nalpha filled: {}\nluma filled: {}\ncolor-keyed filled: {}\n\nSame source image, three extraction configs.",
            diagnostics.thresholds.alpha_filled,
            diagnostics.thresholds.luma_filled,
            diagnostics.thresholds.keyed_filled,
        ),
        LabView::Atlas => format!(
            "View: Atlas (3)\natlas tiles: {}\nnon-empty tiles: {}\ntotal tile contours: {}\n\nExercises AtlasSlicer row-major extraction.",
            diagnostics.atlas.atlas_tiles,
            diagnostics.atlas.non_empty_tiles,
            diagnostics.atlas.total_tile_contours,
        ),
        LabView::Composite => format!(
            "View: Composite (4)\nplaced tiles: {}\nper-tile contours: {}\ncomposite contours: {}\ncomposite convex pieces: {}\n\nLeft side shows per-tile seams, right side shows merged output from one stamped mask.",
            diagnostics.composite.placed_tiles,
            diagnostics.composite.per_tile_contours,
            diagnostics.composite.composite_contours,
            diagnostics.composite.composite_pieces,
        ),
        LabView::Destructible => format!(
            "View: Destructible (5)\nblasts applied: {}\nregenerations: {}\ninitial filled pixels: {}\ncurrent filled pixels: {}\ninitial contours: {}\ncurrent contours: {}\ncurrent convex pieces: {}\nwarnings: {}\n\nThe source BinaryImage is mutated in ECS and regenerated with ColliderGenDirty.",
            diagnostics.destructible.blasts_applied,
            diagnostics.destructible.regenerations,
            diagnostics.destructible.initial_filled_pixels,
            diagnostics.destructible.current_filled_pixels,
            diagnostics.destructible.initial_contours,
            diagnostics.destructible.current_contours,
            diagnostics.destructible.current_pieces,
            diagnostics.destructible.warnings,
        ),
    };

    *text = Text::new(body);
}

fn draw_active_scene(
    mut gizmos: Gizmos,
    active_view: Res<ActiveView>,
    overview: Res<OverviewScene>,
    thresholds: Res<ThresholdScene>,
    atlas: Res<AtlasScene>,
    composite: Res<CompositeScene>,
    destructible: Query<
        (&ColliderGenSource, Option<&ColliderGenOutput>),
        With<LabDestructibleSource>,
    >,
) {
    match active_view.current {
        LabView::Overview => draw_overview_scene(&mut gizmos, &overview),
        LabView::Thresholds => draw_threshold_scene(&mut gizmos, &thresholds),
        LabView::Atlas => draw_atlas_scene(&mut gizmos, &atlas),
        LabView::Composite => draw_composite_scene(&mut gizmos, &composite),
        LabView::Destructible => {
            let Ok((source, output)) = destructible.single() else {
                return;
            };
            draw_destructible_scene(&mut gizmos, source, output);
        }
    }
}

fn draw_overview_scene(gizmos: &mut Gizmos<'_, '_>, scene: &OverviewScene) {
    let left_offset = Vec2::new(-360.0, 0.0);
    let right_offset = Vec2::new(360.0, 0.0);

    draw_panel_frame(
        gizmos,
        left_offset,
        Vec2::new(560.0, 420.0),
        Color::srgba(1.0, 1.0, 1.0, 0.12),
    );
    draw_panel_frame(
        gizmos,
        right_offset,
        Vec2::new(560.0, 420.0),
        Color::srgba(1.0, 1.0, 1.0, 0.12),
    );
    draw_mask_at(
        gizmos,
        &scene.mask,
        scene.transform,
        left_offset,
        Color::srgba(0.27, 0.31, 0.36, 0.32),
    );
    draw_mask_at(
        gizmos,
        &scene.mask,
        scene.transform,
        right_offset,
        Color::srgba(0.27, 0.31, 0.36, 0.32),
    );

    for contour in &scene.raw_contours {
        draw_loop_at(
            gizmos,
            &contour.points,
            left_offset,
            Color::srgba(0.84, 0.86, 0.91, 0.65),
        );
    }
    for contour in &scene.pixel_result.contours {
        draw_loop_at(
            gizmos,
            &contour.points,
            left_offset,
            Color::srgb(0.20, 0.97, 0.84),
        );
    }
    for (index, piece) in scene.pixel_result.convex_pieces.iter().enumerate() {
        let points = piece
            .points
            .iter()
            .map(|point| *point + piece.offset)
            .collect::<Vec<_>>();
        draw_loop_at(gizmos, &points, left_offset, palette(index));
    }

    for contour in &scene.marching_result.contours {
        draw_loop_at(
            gizmos,
            &contour.points,
            right_offset,
            Color::srgb(0.98, 0.58, 0.26),
        );
    }
    for hull in &scene.marching_result.convex_hulls {
        draw_loop_at(
            gizmos,
            &hull.points,
            right_offset,
            Color::srgba(0.96, 0.84, 0.36, 0.88),
        );
    }
}

fn draw_threshold_scene(gizmos: &mut Gizmos<'_, '_>, scene: &ThresholdScene) {
    for (index, panel) in scene.panels.iter().enumerate() {
        let transform = CoordinateTransform::centered(
            panel.mask.width(),
            panel.mask.height(),
            Vec2::splat(LAB_SCALE * 1.4),
        );
        draw_panel_frame(
            gizmos,
            panel.offset,
            Vec2::new(320.0, 320.0),
            palette(index).with_alpha(0.24),
        );
        draw_mask_at(
            gizmos,
            &panel.mask,
            transform,
            panel.offset,
            palette(index).with_alpha(0.56),
        );
    }
}

fn draw_atlas_scene(gizmos: &mut Gizmos<'_, '_>, scene: &AtlasScene) {
    for (index, tile) in scene.tiles.iter().enumerate() {
        let transform = CoordinateTransform::centered(
            tile.mask.width(),
            tile.mask.height(),
            Vec2::splat(LAB_SCALE * 0.95),
        );
        draw_panel_frame(
            gizmos,
            tile.offset,
            Vec2::new(300.0, 240.0),
            palette(index).with_alpha(0.24),
        );
        draw_mask_at(
            gizmos,
            &tile.mask,
            transform,
            tile.offset,
            Color::srgba(0.30, 0.34, 0.38, 0.32),
        );
        for contour in &tile.result.contours {
            draw_loop_at(gizmos, &contour.points, tile.offset, palette(index));
        }
    }
}

fn draw_composite_scene(gizmos: &mut Gizmos<'_, '_>, scene: &CompositeScene) {
    let left_offset = Vec2::new(-360.0, 0.0);
    let right_offset = Vec2::new(360.0, 0.0);

    draw_panel_frame(
        gizmos,
        left_offset,
        Vec2::new(560.0, 420.0),
        Color::srgba(1.0, 1.0, 1.0, 0.12),
    );
    draw_panel_frame(
        gizmos,
        right_offset,
        Vec2::new(560.0, 420.0),
        Color::srgba(1.0, 1.0, 1.0, 0.12),
    );
    draw_mask_at(
        gizmos,
        &scene.mask,
        scene.transform,
        left_offset,
        Color::srgba(0.28, 0.32, 0.38, 0.28),
    );
    draw_mask_at(
        gizmos,
        &scene.mask,
        scene.transform,
        right_offset,
        Color::srgba(0.28, 0.32, 0.38, 0.28),
    );
    for (index, contour) in scene.per_tile_contours.iter().enumerate() {
        draw_loop_at(gizmos, contour, left_offset, palette(index));
    }
    for contour in &scene.composite_result.contours {
        draw_loop_at(
            gizmos,
            &contour.points,
            right_offset,
            Color::srgb(0.18, 0.95, 0.88),
        );
    }
    for (index, piece) in scene.composite_result.convex_pieces.iter().enumerate() {
        let points = piece
            .points
            .iter()
            .map(|point| *point + piece.offset)
            .collect::<Vec<_>>();
        draw_loop_at(gizmos, &points, right_offset, palette(index));
    }
}

fn draw_destructible_scene(
    gizmos: &mut Gizmos<'_, '_>,
    source: &ColliderGenSource,
    output: Option<&ColliderGenOutput>,
) {
    let ColliderGenSourceKind::Binary(mask) = &source.kind else {
        return;
    };
    let transform =
        CoordinateTransform::centered(mask.width(), mask.height(), Vec2::splat(LAB_SCALE));
    draw_panel_frame(
        gizmos,
        Vec2::ZERO,
        Vec2::new(620.0, 420.0),
        Color::srgba(1.0, 1.0, 1.0, 0.12),
    );
    draw_mask(
        gizmos,
        mask,
        transform,
        Color::srgba(0.29, 0.33, 0.37, 0.32),
    );
    if let Some(output) = output {
        for contour in &output.result.contours {
            draw_loop(gizmos, &contour.points, Color::srgb(0.22, 0.96, 0.82));
        }
        for (index, piece) in output.result.convex_pieces.iter().enumerate() {
            let points = piece
                .points
                .iter()
                .map(|point| *point + piece.offset)
                .collect::<Vec<_>>();
            draw_loop(gizmos, &points, palette(index));
        }
        if output
            .result
            .warnings
            .contains(&ColliderGenWarning::HoleAwareDecompositionRecommended)
        {
            gizmos.rect_2d(
                Isometry2d::from_translation(Vec2::new(0.0, 170.0)),
                Vec2::new(240.0, 24.0),
                Color::srgb(0.98, 0.66, 0.26),
            );
        }
    }
}

fn draw_mask(
    gizmos: &mut Gizmos<'_, '_>,
    mask: &BinaryImage,
    transform: CoordinateTransform,
    color: Color,
) {
    draw_mask_at(gizmos, mask, transform, Vec2::ZERO, color);
}

fn draw_mask_at(
    gizmos: &mut Gizmos<'_, '_>,
    mask: &BinaryImage,
    transform: CoordinateTransform,
    offset: Vec2,
    color: Color,
) {
    for y in 0..mask.height() {
        for x in 0..mask.width() {
            if !mask.get(x, y) {
                continue;
            }

            let min = transform
                .pixel_to_local(Vec2::new(x as f32, mask.height() as f32 - y as f32 - 1.0))
                + offset;
            let max = transform
                .pixel_to_local(Vec2::new(x as f32 + 1.0, mask.height() as f32 - y as f32))
                + offset;
            draw_loop(
                gizmos,
                &[
                    Vec2::new(min.x, min.y),
                    Vec2::new(max.x, min.y),
                    Vec2::new(max.x, max.y),
                    Vec2::new(min.x, max.y),
                ],
                color,
            );
        }
    }
}

fn draw_panel_frame(gizmos: &mut Gizmos<'_, '_>, center: Vec2, size: Vec2, color: Color) {
    gizmos.rect_2d(Isometry2d::from_translation(center), size, color);
}

fn draw_loop(gizmos: &mut Gizmos<'_, '_>, points: &[Vec2], color: Color) {
    draw_loop_at(gizmos, points, Vec2::ZERO, color);
}

fn draw_loop_at(gizmos: &mut Gizmos<'_, '_>, points: &[Vec2], offset: Vec2, color: Color) {
    if points.len() < 2 {
        return;
    }
    for index in 0..points.len() {
        gizmos.line_2d(
            points[index] + offset,
            points[(index + 1) % points.len()] + offset,
            color,
        );
    }
}

fn tile_translation(origin: UVec2, tile_size: UVec2, full_size: UVec2, scale: Vec2) -> Vec2 {
    Vec2::new(
        (origin.x as f32 + tile_size.x as f32 * 0.5 - full_size.x as f32 * 0.5) * scale.x,
        (origin.y as f32 + tile_size.y as f32 * 0.5 - full_size.y as f32 * 0.5) * scale.y,
    )
}

fn palette(index: usize) -> Color {
    const COLORS: [Color; 6] = [
        Color::srgb(0.95, 0.36, 0.28),
        Color::srgb(0.95, 0.74, 0.19),
        Color::srgb(0.18, 0.80, 0.55),
        Color::srgb(0.18, 0.60, 0.95),
        Color::srgb(0.70, 0.44, 0.96),
        Color::srgb(0.97, 0.56, 0.23),
    ];
    COLORS[index % COLORS.len()]
}
