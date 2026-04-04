use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    BinaryImage, ColliderGenConfig, ColliderGenDirty, ColliderGenLod, ColliderGenOutput,
    ColliderGenPlugin, ColliderGenSource, ColliderGenSourceKind, CoordinateTransform,
};
use support::ColliderGenExamplePane;

#[derive(Resource)]
struct BlastTimer(Timer);

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen destructible");
    app.insert_resource(BlastTimer(Timer::from_seconds(0.9, TimerMode::Repeating)))
        .add_plugins(ColliderGenPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                sync_destructible_pane,
                carve_terrain,
                draw_scene,
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    let mut mask = BinaryImage::new(28, 18);
    mask.fill_rect(0, 0, 28, 6);
    mask.fill_rect(4, 8, 6, 2);
    mask.fill_rect(14, 10, 10, 2);

    commands.spawn((
        Name::new("DestructibleTerrain"),
        ColliderGenSource {
            kind: ColliderGenSourceKind::Binary(mask),
            config: pane_config(&pane),
        },
    ));
}

fn pane_config(pane: &ColliderGenExamplePane) -> ColliderGenConfig {
    support::pane_config(pane, ColliderGenLod::Medium)
}

fn sync_destructible_pane(
    pane: Res<ColliderGenExamplePane>,
    mut timer: ResMut<BlastTimer>,
    mut sources: Query<&mut ColliderGenSource>,
) {
    timer.0
        .set_duration(std::time::Duration::from_secs_f32(pane.cycle_seconds.max(0.05)));
    for mut source in &mut sources {
        source.config = pane_config(&pane);
    }
}

fn carve_terrain(
    mut commands: Commands,
    time: Res<Time>,
    pane: Res<ColliderGenExamplePane>,
    mut timer: ResMut<BlastTimer>,
    mut query: Query<(Entity, &mut ColliderGenSource)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    for (entity, mut source) in &mut query {
        if let ColliderGenSourceKind::Binary(mask) = &mut source.kind {
            let phase = time.elapsed_secs();
            let center = IVec2::new(
                6 + (phase.sin().abs() * 16.0) as i32,
                4 + (phase.cos().abs() * 8.0) as i32,
            );
            let blast_radius = pane.blast_radius.round().max(1.0) as i32;
            mask.carve_circle(center, blast_radius);
            let dirty_radius = blast_radius + 1;
            commands.entity(entity).insert(ColliderGenDirty {
                region: Some(IRect::new(
                    center.x - dirty_radius,
                    center.y - dirty_radius,
                    center.x + dirty_radius,
                    center.y + dirty_radius,
                )),
            });
        }
    }
}

fn draw_scene(
    mut gizmos: Gizmos,
    query: Query<(&ColliderGenSource, Option<&ColliderGenOutput>)>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    for (source, output) in &query {
        let ColliderGenSourceKind::Binary(mask) = &source.kind else {
            continue;
        };
        let transform =
            CoordinateTransform::centered(mask.width(), mask.height(), Vec2::splat(pane.render_scale));
        if pane.show_mask {
            support::draw_mask(
                &mut gizmos,
                mask,
                transform,
                Color::srgba(0.30, 0.34, 0.38, 0.35),
            );
        }
        if let Some(output) = output {
            for contour in &output.result.contours {
                support::draw_contour(&mut gizmos, contour, Color::srgb(0.18, 0.94, 0.85));
            }
            if pane.show_hulls {
                for hull in &output.result.convex_hulls {
                    support::draw_contour(&mut gizmos, hull, Color::srgb(0.96, 0.58, 0.22));
                }
            }
            if pane.show_pieces {
                for (index, piece) in output.result.convex_pieces.iter().enumerate() {
                    support::draw_piece(&mut gizmos, piece, support::palette(index));
                }
            }
            support::update_result_stats(&mut pane, &output.result);
        }
    }
}
