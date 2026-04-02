use bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{LabDiagnostics, LabView};

use super::switch_view;

pub fn build() -> Scenario {
    Scenario::builder("collider_gen_atlas")
        .description("Verify atlas slicing produces multiple non-empty tile archetypes")
        .then(switch_view(LabView::Atlas))
        .then(Action::WaitFrames(8))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "atlas view active",
            |diagnostics| diagnostics.active_view == LabView::Atlas,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "atlas produced non-empty tiles and contours",
            |diagnostics| {
                diagnostics.atlas.atlas_tiles == 6
                    && diagnostics.atlas.non_empty_tiles >= 4
                    && diagnostics.atlas.total_tile_contours >= diagnostics.atlas.non_empty_tiles
            },
        ))
        .then(Action::Screenshot("atlas_tiles".into()))
        .then(assertions::log_summary("collider_gen_atlas"))
        .build()
}
