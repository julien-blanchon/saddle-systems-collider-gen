use std::env;

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    BinaryImage, ColliderGenResult, CompoundPolygon, Contour, CoordinateTransform,
};

#[derive(Resource)]
struct ExampleAutoExit(Timer);

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
    .add_systems(Startup, setup_camera);
    install_auto_exit(app);
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
