use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    BinaryImage, ColliderGenDirty, ColliderGenOutput, ColliderGenPlugin, ColliderGenSource,
    ColliderGenSourceKind, CoordinateTransform,
};

#[derive(Resource)]
struct BlastTimer(Timer);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "collider_gen destructible".to_string(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)))
    .insert_resource(BlastTimer(Timer::from_seconds(0.9, TimerMode::Repeating)))
    .add_plugins(ColliderGenPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, (carve_terrain, draw_scene));
    support::install_auto_exit(&mut app);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Name::new("ExampleCamera"), Camera2d));

    let mut mask = BinaryImage::new(28, 18);
    mask.fill_rect(0, 0, 28, 6);
    mask.fill_rect(4, 8, 6, 2);
    mask.fill_rect(14, 10, 10, 2);

    commands.spawn((
        Name::new("DestructibleTerrain"),
        ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(mask),
            config: default(),
        },
    ));
}

fn carve_terrain(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<BlastTimer>,
    mut query: Query<(Entity, &mut ColliderGenSource)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    for (entity, mut source) in &mut query {
        if let ColliderGenSourceKind::Binary(mask) = &mut source.kind {
            let phase = timer.0.elapsed_secs();
            let center = IVec2::new(
                6 + (phase.sin().abs() * 16.0) as i32,
                4 + (phase.cos().abs() * 8.0) as i32,
            );
            mask.carve_circle(center, 2);
            commands.entity(entity).insert(ColliderGenDirty {
                region: Some(IRect::new(
                    center.x - 3,
                    center.y - 3,
                    center.x + 3,
                    center.y + 3,
                )),
            });
        }
    }
}

fn draw_scene(mut gizmos: Gizmos, query: Query<(&ColliderGenSource, Option<&ColliderGenOutput>)>) {
    for (source, output) in &query {
        let ColliderGenSourceKind::Binary(mask) = &source.kind else {
            continue;
        };
        let transform =
            CoordinateTransform::centered(mask.width(), mask.height(), Vec2::splat(20.0));
        support::draw_mask(
            &mut gizmos,
            mask,
            transform,
            Color::srgba(0.30, 0.34, 0.38, 0.35),
        );
        if let Some(output) = output {
            support::draw_result(
                &mut gizmos,
                &output.result,
                Color::srgb(0.18, 0.94, 0.85),
                Color::srgb(0.96, 0.58, 0.22),
            );
        }
    }
}
