use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    AtlasColliderFrame, AtlasSlicer, BinaryImage, ColliderGenLod, CoordinateTransform,
    bake_atlas_collider_frames,
};
use support::{ColliderGenExamplePane, ColliderGenPaneSettings};

#[derive(Resource)]
struct FrameData {
    masks: Vec<BinaryImage>,
    frames: Vec<AtlasColliderFrame>,
    index: usize,
    timer: Timer,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen animation frames");
    app.add_systems(Startup, setup)
        .add_systems(Update, (refresh_scene, advance_frame, draw_scene).chain())
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    commands.insert_resource(build_frame_data(&pane));
}

fn build_frame_data(pane: &ColliderGenExamplePane) -> FrameData {
    let mut sheet = BinaryImage::new(32, 8);
    for frame in 0..4u32 {
        let offset = frame * 8;
        sheet.fill_rect(offset + 1, 1, 3 + frame, 4);
        sheet.fill_rect(offset + 1, 5, 1, 2);
    }

    let slicer = AtlasSlicer::from_grid(sheet, UVec2::new(8, 8), 4, 1, None, None);
    let masks = (0..4)
        .map(|index| slicer.slice_index(index).expect("frame should slice"))
        .collect();
    let frames =
        bake_atlas_collider_frames(&slicer, &support::pane_config(pane, ColliderGenLod::High))
            .expect("frame bake should succeed");

    FrameData {
        masks,
        frames,
        index: 0,
        timer: Timer::from_seconds(pane.cycle_seconds.max(0.05), TimerMode::Repeating),
    }
}

fn refresh_scene(
    pane: Res<ColliderGenExamplePane>,
    mut frame_data: ResMut<FrameData>,
    mut last_settings: Local<Option<ColliderGenPaneSettings>>,
) {
    let settings = support::pane_settings(&pane);
    if last_settings.as_ref() == Some(&settings) {
        frame_data
            .timer
            .set_duration(std::time::Duration::from_secs_f32(
                pane.cycle_seconds.max(0.05),
            ));
        return;
    }

    *frame_data = build_frame_data(&pane);
    *last_settings = Some(settings);
}

fn advance_frame(time: Res<Time>, mut frame_data: ResMut<FrameData>) {
    frame_data.timer.tick(time.delta());
    if frame_data.timer.just_finished() {
        frame_data.index = (frame_data.index + 1) % frame_data.frames.len();
    }
}

fn draw_scene(
    mut gizmos: Gizmos,
    frame_data: Res<FrameData>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    let mask = &frame_data.masks[frame_data.index];
    let result = &frame_data.frames[frame_data.index].result;
    let transform =
        CoordinateTransform::centered(mask.width(), mask.height(), Vec2::splat(pane.render_scale));
    if pane.show_mask {
        support::draw_mask(
            &mut gizmos,
            mask,
            transform,
            Color::srgba(0.42, 0.44, 0.48, 0.4),
        );
    }
    for contour in &result.contours {
        support::draw_contour(&mut gizmos, contour, Color::srgb(0.22, 0.96, 0.74));
    }
    if pane.show_hulls {
        for hull in &result.convex_hulls {
            support::draw_contour(&mut gizmos, hull, Color::srgb(0.95, 0.50, 0.32));
        }
    }
    if pane.show_pieces {
        for (index, piece) in result.convex_pieces.iter().enumerate() {
            support::draw_piece(&mut gizmos, piece, support::palette(index));
        }
    }
    support::update_result_stats(&mut pane, result);
}
