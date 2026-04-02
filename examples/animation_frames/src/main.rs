use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{
    AtlasSlicer, BinaryImage, ColliderGenConfig, CoordinateTransform,
};

#[derive(Resource)]
struct FrameData {
    frames: Vec<(BinaryImage, saddle_systems_collider_gen::ColliderGenResult)>,
    index: usize,
    timer: Timer,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen animation frames");
    app.add_systems(Startup, setup)
        .add_systems(Update, (advance_frame, draw_scene))
        .run();
}

fn setup(mut commands: Commands) {
    let mut sheet = BinaryImage::new(32, 8);
    for frame in 0..4u32 {
        let offset = frame * 8;
        sheet.fill_rect(offset + 1, 1, 3 + frame, 4);
        sheet.fill_rect(offset + 1, 5, 1, 2);
    }

    let slicer = AtlasSlicer::from_grid(sheet, UVec2::new(8, 8), 4, 1, None, None);
    let mut frames = Vec::new();
    for index in 0..4 {
        let frame = slicer.slice_index(index).expect("frame should slice");
        let result = saddle_systems_collider_gen::generate_collider_geometry(
            &frame,
            &ColliderGenConfig::default(),
        )
        .expect("frame should generate");
        frames.push((frame, result));
    }

    commands.insert_resource(FrameData {
        frames,
        index: 0,
        timer: Timer::from_seconds(0.35, TimerMode::Repeating),
    });
}

fn advance_frame(time: Res<Time>, mut frame_data: ResMut<FrameData>) {
    frame_data.timer.tick(time.delta());
    if frame_data.timer.just_finished() {
        frame_data.index = (frame_data.index + 1) % frame_data.frames.len();
    }
}

fn draw_scene(mut gizmos: Gizmos, frame_data: Res<FrameData>) {
    let (mask, result) = &frame_data.frames[frame_data.index];
    let transform = CoordinateTransform::centered(mask.width(), mask.height(), Vec2::splat(36.0));
    support::draw_mask(
        &mut gizmos,
        mask,
        transform,
        Color::srgba(0.42, 0.44, 0.48, 0.4),
    );
    support::draw_result(
        &mut gizmos,
        result,
        Color::srgb(0.22, 0.96, 0.74),
        Color::srgb(0.95, 0.50, 0.32),
    );
}
