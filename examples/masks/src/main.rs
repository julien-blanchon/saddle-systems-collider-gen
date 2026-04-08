use saddle_systems_collider_gen_example_support as support;

use bevy::prelude::*;
use saddle_systems_collider_gen::{BinaryImage, CoordinateTransform};
use support::{ColliderGenExamplePane, ColliderGenPaneSettings};

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
        .add_systems(Update, (refresh_scene, draw_scene).chain())
        .run();
}

fn setup(mut commands: Commands, pane: Res<ColliderGenExamplePane>) {
    commands.insert_resource(build_variants(&pane));
}

fn build_variants(pane: &ColliderGenExamplePane) -> MaskVariants {
    let mut original = BinaryImage::new(18, 18);
    original.fill_circle(IVec2::new(9, 9), 7);
    original.carve_circle(IVec2::new(9, 9), 3);
    original.set(2, 2, true);
    original.set(15, 15, true);
    original.set(9, 1, false);
    original.set(9, 16, false);

    let radius = pane.morphology_radius;
    let opened = original.open(radius);
    let closed = original.close(radius);
    MaskVariants {
        original,
        opened,
        closed,
    }
}

fn refresh_scene(
    pane: Res<ColliderGenExamplePane>,
    mut variants: ResMut<MaskVariants>,
    mut last_settings: Local<Option<ColliderGenPaneSettings>>,
) {
    let settings = support::pane_settings(&pane);
    if last_settings.as_ref() == Some(&settings) {
        return;
    }

    *variants = build_variants(&pane);
    *last_settings = Some(settings);
}

fn draw_scene(
    mut gizmos: Gizmos,
    variants: Res<MaskVariants>,
    mut pane: ResMut<ColliderGenExamplePane>,
) {
    for (index, mask) in [&variants.original, &variants.opened, &variants.closed]
        .into_iter()
        .enumerate()
    {
        let offset = Vec2::new(index as f32 * 320.0 - 320.0, 0.0);
        let transform = CoordinateTransform::centered(
            mask.width(),
            mask.height(),
            Vec2::splat(pane.render_scale),
        );
        if pane.show_mask {
            support::draw_mask_at(
                &mut gizmos,
                mask,
                transform,
                offset,
                support::palette(index).with_alpha(0.75),
            );
        }
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

    pane.contour_count = 3;
    pane.hull_count = 0;
    pane.piece_count = 0;
    pane.warning_count = 0;
}
