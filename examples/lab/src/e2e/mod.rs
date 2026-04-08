mod atlas;
mod composite;
mod destructible;
mod smoke;
mod thresholds;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, scenario::Scenario};

use crate::{LabView, set_active_view};

pub struct ColliderGenLabE2EPlugin;

impl Plugin for ColliderGenLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(saddle_bevy_e2e::E2EPlugin);

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                saddle_bevy_e2e::init_scenario(app, scenario);
            } else {
                error!(
                    "[saddle-systems-collider-gen-lab:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}

pub(crate) fn switch_view(view: LabView) -> Action {
    Action::Custom(Box::new(move |world: &mut World| {
        set_active_view(world, view);
    }))
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "collider_gen_basic" => Some(build_basic()),
        "collider_gen_smoke" => Some(smoke::build()),
        "collider_gen_thresholds" => Some(thresholds::build()),
        "collider_gen_atlas" => Some(atlas::build()),
        "collider_gen_composite" => Some(composite::build()),
        "collider_gen_destructible" => Some(destructible::build()),
        "collider_gen_masks" => Some(build_masks()),
        "collider_gen_tilemap_merge" => Some(build_tilemap_merge()),
        "collider_gen_debug_gizmos" => Some(build_debug_gizmos()),
        "collider_gen_animation_frames" => Some(build_animation_frames()),
        _ => None,
    }
}

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "collider_gen_basic",
        "collider_gen_smoke",
        "collider_gen_thresholds",
        "collider_gen_atlas",
        "collider_gen_composite",
        "collider_gen_destructible",
        "collider_gen_masks",
        "collider_gen_tilemap_merge",
        "collider_gen_debug_gizmos",
        "collider_gen_animation_frames",
    ]
}

fn build_basic() -> Scenario {
    use crate::LabDiagnostics;
    use saddle_bevy_e2e::actions::assertions;

    Scenario::builder("collider_gen_basic")
        .description(
            "Stay on the Overview view and verify the baseline authored-mask workflow produces \
             simplified contours and convex pieces, matching the intent of the basic example.",
        )
        .then(switch_view(LabView::Overview))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "overview view is active",
            |diagnostics| diagnostics.active_view == LabView::Overview,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "basic authored mask produced collider outputs",
            |diagnostics| {
                diagnostics.overview.simplified_contours > 0
                    && diagnostics.overview.convex_pieces > 0
            },
        ))
        .then(Action::Screenshot("collider_gen_basic".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("collider_gen_basic"))
        .build()
}

fn build_masks() -> Scenario {
    use crate::LabDiagnostics;
    use saddle_bevy_e2e::actions::assertions;

    Scenario::builder("collider_gen_masks")
        .description(
            "Switch to the Thresholds view and verify that the three threshold extraction \
             strategies (alpha, luma, color-key) each produce distinct non-zero pixel \
             populations, confirming mask divergence across strategies.",
        )
        .then(switch_view(LabView::Thresholds))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "thresholds view is active",
            |diagnostics| diagnostics.active_view == LabView::Thresholds,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "alpha mask produces non-zero filled pixels",
            |diagnostics| diagnostics.thresholds.alpha_filled > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "luma mask produces non-zero filled pixels",
            |diagnostics| diagnostics.thresholds.luma_filled > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "keyed mask produces non-zero filled pixels",
            |diagnostics| diagnostics.thresholds.keyed_filled > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "at least two mask strategies differ from each other",
            |diagnostics| {
                diagnostics.thresholds.alpha_filled != diagnostics.thresholds.luma_filled
                    || diagnostics.thresholds.alpha_filled != diagnostics.thresholds.keyed_filled
            },
        ))
        .then(Action::Screenshot("collider_gen_masks".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("collider_gen_masks"))
        .build()
}

fn build_tilemap_merge() -> Scenario {
    use crate::LabDiagnostics;
    use saddle_bevy_e2e::actions::assertions;

    Scenario::builder("collider_gen_tilemap_merge")
        .description(
            "Switch to the Composite view and verify that stamped tile masks collapse internal \
             seams into fewer merged contours, matching the tilemap_merge example workflow.",
        )
        .then(switch_view(LabView::Composite))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "composite view is active",
            |diagnostics| diagnostics.active_view == LabView::Composite,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "tilemap merge reduced per-tile seam count",
            |diagnostics| {
                diagnostics.composite.placed_tiles > 0
                    && diagnostics.composite.per_tile_contours
                        > diagnostics.composite.composite_contours
                    && diagnostics.composite.composite_pieces > 0
            },
        ))
        .then(Action::Screenshot("collider_gen_tilemap_merge".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("collider_gen_tilemap_merge"))
        .build()
}

fn build_debug_gizmos() -> Scenario {
    use crate::LabDiagnostics;
    use saddle_bevy_e2e::actions::assertions;

    Scenario::builder("collider_gen_debug_gizmos")
        .description(
            "Verify the Overview view shows all four geometry representations (raw contours, \
             simplified contours, marching-squares contours, and convex pieces) so that \
             debug gizmo data is populated and renderable.",
        )
        .then(switch_view(LabView::Overview))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "overview view is active",
            |diagnostics| diagnostics.active_view == LabView::Overview,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "raw contours present for gizmo rendering",
            |diagnostics| diagnostics.overview.raw_contours > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "simplified contours present for gizmo rendering",
            |diagnostics| diagnostics.overview.simplified_contours > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "marching-squares contours present for gizmo rendering",
            |diagnostics| diagnostics.overview.marching_contours > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "convex decomposition pieces present for gizmo rendering",
            |diagnostics| diagnostics.overview.convex_pieces > 0,
        ))
        .then(Action::Screenshot("collider_gen_debug_gizmos".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("collider_gen_debug_gizmos"))
        .build()
}

fn build_animation_frames() -> Scenario {
    use crate::LabDiagnostics;
    use saddle_bevy_e2e::actions::assertions;

    Scenario::builder("collider_gen_animation_frames")
        .description(
            "Switch to the Atlas view and verify per-tile collider extraction: at least one \
             atlas tile is non-empty and the total tile contour count is positive, confirming \
             frame-by-frame atlas slicing produces per-frame geometry.",
        )
        .then(switch_view(LabView::Atlas))
        .then(Action::WaitFrames(10))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "atlas view is active",
            |diagnostics| diagnostics.active_view == LabView::Atlas,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "atlas has at least one tile",
            |diagnostics| diagnostics.atlas.atlas_tiles > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "at least one atlas tile is non-empty",
            |diagnostics| diagnostics.atlas.non_empty_tiles > 0,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "total tile contour count is positive (per-frame geometry extracted)",
            |diagnostics| diagnostics.atlas.total_tile_contours > 0,
        ))
        .then(Action::Screenshot("collider_gen_animation_frames".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("collider_gen_animation_frames"))
        .build()
}
