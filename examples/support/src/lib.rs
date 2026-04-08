use std::env;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_systems_collider_gen::{
    BinaryImage, ColliderGenConfig, ColliderGenLod, ColliderGenResult, CompoundPolygon, Contour,
    ContourMode, CoordinateTransform,
};

#[derive(Resource)]
struct ExampleAutoExit(Timer);

#[derive(Resource, Debug, Clone, PartialEq, Pane)]
#[pane(title = "Collider Gen", position = "top-right")]
pub struct ColliderGenExamplePane {
    #[pane(folder = "Generation", slider, min = 8.0, max = 42.0, step = 1.0)]
    pub render_scale: f32,
    #[pane(folder = "Generation", toggle)]
    pub use_marching_squares: bool,
    #[pane(folder = "Generation", toggle)]
    pub decompose: bool,
    #[pane(folder = "Generation", slider, min = 0.0, max = 2.0, step = 0.05)]
    pub rdp_epsilon: f32,
    #[pane(folder = "Generation", slider, min = 0.0, max = 0.4, step = 0.01)]
    pub visvalingam_area_threshold: f32,
    #[pane(folder = "Generation", slider, min = 0.0, max = 4.0, step = 0.1)]
    pub minimum_area: f32,
    #[pane(folder = "Examples", slider, min = 0.0, max = 4.0, step = 1.0)]
    pub morphology_radius: u32,
    #[pane(folder = "Examples", slider, min = 0.12, max = 1.2, step = 0.02)]
    pub cycle_seconds: f32,
    #[pane(folder = "Examples", slider, min = 1.0, max = 5.0, step = 0.25)]
    pub blast_radius: f32,
    #[pane(folder = "Examples", slider, min = 1.0, max = 5.0, step = 1.0)]
    pub lab_view: u32,
    #[pane(folder = "Visuals", toggle)]
    pub show_mask: bool,
    #[pane(folder = "Visuals", toggle)]
    pub show_hulls: bool,
    #[pane(folder = "Visuals", toggle)]
    pub show_pieces: bool,
    #[pane(monitor)]
    pub contour_count: u32,
    #[pane(monitor)]
    pub hull_count: u32,
    #[pane(monitor)]
    pub piece_count: u32,
    #[pane(monitor)]
    pub warning_count: u32,
}

impl Default for ColliderGenExamplePane {
    fn default() -> Self {
        let config = ColliderGenConfig::default();
        Self {
            render_scale: 20.0,
            use_marching_squares: false,
            decompose: config.decomposition.enabled,
            rdp_epsilon: config.simplification.rdp_epsilon,
            visvalingam_area_threshold: config.simplification.visvalingam_area_threshold,
            minimum_area: config.minimum_area,
            morphology_radius: 1,
            cycle_seconds: 0.35,
            blast_radius: 2.0,
            lab_view: 1,
            show_mask: true,
            show_hulls: true,
            show_pieces: true,
            contour_count: 0,
            hull_count: 0,
            piece_count: 0,
            warning_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColliderGenPaneSettings {
    pub render_scale: f32,
    pub use_marching_squares: bool,
    pub decompose: bool,
    pub rdp_epsilon: f32,
    pub visvalingam_area_threshold: f32,
    pub minimum_area: f32,
    pub morphology_radius: u32,
    pub cycle_seconds: f32,
    pub blast_radius: f32,
    pub lab_view: u32,
    pub show_mask: bool,
    pub show_hulls: bool,
    pub show_pieces: bool,
}

pub fn pane_settings(pane: &ColliderGenExamplePane) -> ColliderGenPaneSettings {
    ColliderGenPaneSettings {
        render_scale: pane.render_scale,
        use_marching_squares: pane.use_marching_squares,
        decompose: pane.decompose,
        rdp_epsilon: pane.rdp_epsilon,
        visvalingam_area_threshold: pane.visvalingam_area_threshold,
        minimum_area: pane.minimum_area,
        morphology_radius: pane.morphology_radius,
        cycle_seconds: pane.cycle_seconds,
        blast_radius: pane.blast_radius,
        lab_view: pane.lab_view,
        show_mask: pane.show_mask,
        show_hulls: pane.show_hulls,
        show_pieces: pane.show_pieces,
    }
}

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn configure_app(app: &mut App, title: &str) {
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.to_string(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)))
    .init_resource::<ColliderGenExamplePane>()
    .add_plugins(pane_plugins())
    .register_pane::<ColliderGenExamplePane>()
    .add_systems(Startup, setup_camera);
    install_auto_exit(app);
}

pub fn pane_config(pane: &ColliderGenExamplePane, lod: ColliderGenLod) -> ColliderGenConfig {
    let mut config = ColliderGenConfig::default().with_lod(lod);
    config.scale = Vec2::splat(pane.render_scale.max(1.0));
    config.contour_mode = if pane.use_marching_squares {
        ContourMode::MarchingSquares
    } else {
        ContourMode::PixelExact
    };
    config.simplification.rdp_epsilon = pane.rdp_epsilon.max(0.0);
    config.simplification.visvalingam_area_threshold = pane.visvalingam_area_threshold.max(0.0);
    config.minimum_area = pane.minimum_area.max(0.0);
    config.decomposition.enabled = pane.decompose;
    config
}

pub fn update_result_stats(pane: &mut ColliderGenExamplePane, result: &ColliderGenResult) {
    pane.contour_count = result.contours.len() as u32;
    pane.hull_count = result.convex_hulls.len() as u32;
    pane.piece_count = result.convex_pieces.len() as u32;
    pane.warning_count = result.warnings.len() as u32;
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Name::new("ExampleCamera"), Camera2d));
}

pub fn install_auto_exit(app: &mut App) {
    let Some(seconds) = auto_exit_seconds() else {
        return;
    };

    app.insert_resource(ExampleAutoExit(Timer::from_seconds(
        seconds,
        TimerMode::Once,
    )))
    .add_systems(Update, auto_exit_after_delay);
}

fn auto_exit_seconds() -> Option<f32> {
    env::var("COLLIDER_GEN_AUTO_EXIT_SECS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|seconds| *seconds > 0.0)
}

fn auto_exit_after_delay(
    time: Res<Time>,
    mut timer: ResMut<ExampleAutoExit>,
    mut app_exit: MessageWriter<AppExit>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        app_exit.write(AppExit::Success);
    }
}

pub fn draw_mask(
    gizmos: &mut Gizmos<'_, '_>,
    mask: &BinaryImage,
    transform: CoordinateTransform,
    color: Color,
) {
    draw_mask_at(gizmos, mask, transform, Vec2::ZERO, color);
}

pub fn draw_mask_at(
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
            let bottom_left = Vec2::new(min.x, min.y);
            let bottom_right = Vec2::new(max.x, min.y);
            let top_right = Vec2::new(max.x, max.y);
            let top_left = Vec2::new(min.x, max.y);
            draw_loop(
                gizmos,
                &[bottom_left, bottom_right, top_right, top_left],
                color,
            );
        }
    }
}

pub fn draw_result(
    gizmos: &mut Gizmos<'_, '_>,
    result: &ColliderGenResult,
    contour_color: Color,
    hull_color: Color,
) {
    for contour in &result.contours {
        draw_contour(gizmos, contour, contour_color);
    }
    for hull in &result.convex_hulls {
        draw_contour(gizmos, hull, hull_color);
    }
    for (index, piece) in result.convex_pieces.iter().enumerate() {
        draw_piece(gizmos, piece, palette(index));
    }
}

pub fn draw_contour(gizmos: &mut Gizmos<'_, '_>, contour: &Contour, color: Color) {
    draw_loop(gizmos, &contour.points, color);
}

pub fn draw_piece(gizmos: &mut Gizmos<'_, '_>, piece: &CompoundPolygon, color: Color) {
    let points: Vec<Vec2> = piece
        .points
        .iter()
        .map(|point| *point + piece.offset)
        .collect();
    draw_loop(gizmos, &points, color);
}

pub fn draw_loop(gizmos: &mut Gizmos<'_, '_>, points: &[Vec2], color: Color) {
    if points.len() < 2 {
        return;
    }
    for index in 0..points.len() {
        let start = points[index];
        let end = points[(index + 1) % points.len()];
        gizmos.line_2d(start, end, color);
    }
}

pub fn palette(index: usize) -> Color {
    const COLORS: [Color; 5] = [
        Color::srgb(0.95, 0.34, 0.27),
        Color::srgb(0.96, 0.73, 0.18),
        Color::srgb(0.16, 0.74, 0.52),
        Color::srgb(0.18, 0.58, 0.95),
        Color::srgb(0.74, 0.42, 0.96),
    ];
    COLORS[index % COLORS.len()]
}
