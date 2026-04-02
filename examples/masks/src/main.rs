use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, CoordinateTransform};

#[derive(Resource)]
struct MaskVariants {
    original: BinaryImage,
    opened: BinaryImage,
    closed: BinaryImage,
}

fn main() {
    let mut app = App::new();
    support::configure_app(&mut app, "collider_gen masks");
    app.add_systems(Startup, setup)
        .add_systems(Update, draw_scene)
        .run();
}

fn setup(mut commands: Commands) {
    let mut original = BinaryImage::new(18, 18);
    original.fill_circle(IVec2::new(9, 9), 7);
    original.carve_circle(IVec2::new(9, 9), 3);
    original.set(2, 2, true);
    original.set(15, 15, true);
    original.set(9, 1, false);
    original.set(9, 16, false);

    let opened = original.open(1);
    let closed = original.close(1);
    commands.insert_resource(MaskVariants {
        original,
        opened,
        closed,
    });
}

fn draw_scene(mut gizmos: Gizmos, variants: Res<MaskVariants>) {
    for (index, mask) in [&variants.original, &variants.opened, &variants.closed]
        .into_iter()
        .enumerate()
    {
        let offset = Vec2::new(index as f32 * 320.0 - 320.0, 0.0);
        let transform =
            CoordinateTransform::centered(mask.width(), mask.height(), Vec2::splat(14.0));
        support::draw_mask_at(
            &mut gizmos,
            mask,
            transform,
            offset,
            support::palette(index),
        );
        let bounds = Rect::from_center_size(offset, Vec2::splat(260.0));
        support::draw_loop(
            &mut gizmos,
            &[
                bounds.min,
                Vec2::new(bounds.max.x, bounds.min.y),
                bounds.max,
                Vec2::new(bounds.min.x, bounds.max.y),
            ],
            Color::srgba(1.0, 1.0, 1.0, 0.15),
        );
    }
}
