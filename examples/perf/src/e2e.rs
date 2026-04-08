use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{PerfBenchPane, PerfSummary};

pub struct ColliderGenPerfE2EPlugin;

impl Plugin for ColliderGenPerfE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(saddle_bevy_e2e::E2EPlugin);

        let args: Vec<String> = std::env::args().collect();
        if let Some(name) = parse_e2e_args(&args) {
            if let Some(scenario) = scenario_by_name(&name) {
                saddle_bevy_e2e::init_scenario(app, scenario);
            } else {
                error!(
                    "[saddle-systems-collider-gen-example-perf:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .cloned()
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "collider_gen_perf_smoke" => Some(perf_smoke()),
        _ => None,
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["collider_gen_perf_smoke"]
}

fn perf_smoke() -> Scenario {
    Scenario::builder("collider_gen_perf_smoke")
        .description(
            "Run the perf dashboard with its default bench settings, then tune the sliders \
             programmatically and verify every benchmark summary updates to the new inputs.",
        )
        .then(Action::WaitFrames(5))
        .then(assertions::resource_satisfies::<PerfSummary>(
            "default perf summaries are populated",
            |summary| {
                !summary.full_pipeline.is_empty()
                    && !summary.simplification.is_empty()
                    && !summary.atlas.is_empty()
                    && !summary.dirty_region.is_empty()
            },
        ))
        .then(Action::Screenshot("perf_default".into()))
        .then(Action::Custom(Box::new(|world| {
            let mut pane = world.resource_mut::<PerfBenchPane>();
            pane.mask_size = 256;
            pane.simplification_vertices = 1_200;
            pane.atlas_tiles_per_side = 4;
            pane.dirty_region_size = 1_024;
        })))
        .then(Action::WaitFrames(3))
        .then(assertions::resource_satisfies::<PerfSummary>(
            "perf summaries reflect tuned settings",
            |summary| {
                summary.full_pipeline.contains("256^2")
                    && summary.simplification.contains("1200")
                    && summary.atlas.contains("4x4")
                    && summary.dirty_region.contains("1024^2")
            },
        ))
        .then(Action::Screenshot("perf_tuned".into()))
        .then(assertions::log_summary("collider_gen_perf_smoke"))
        .build()
}
