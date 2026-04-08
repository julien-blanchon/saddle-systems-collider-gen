use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabDiagnostics, LabView};

pub fn build() -> Scenario {
    Scenario::builder("collider_gen_smoke")
        .description("Verify the overview scene renders raw, simplified, and convex-piece outputs")
        .then(Action::WaitFrames(45))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "overview view active",
            |diagnostics| diagnostics.active_view == LabView::Overview,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "overview generated geometry is populated",
            |diagnostics| {
                diagnostics.overview.raw_contours > 0
                    && diagnostics.overview.simplified_contours > 0
                    && diagnostics.overview.marching_contours > 0
                    && diagnostics.overview.convex_pieces > 0
            },
        ))
        .then(inspect::log_resource::<LabDiagnostics>(
            "collider_gen_smoke diagnostics",
        ))
        .then(inspect::log_world_summary("collider_gen_smoke world"))
        .then(Action::Screenshot("overview".into()))
        .then(assertions::log_summary("collider_gen_smoke"))
        .build()
}
