use bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{LabDiagnostics, LabView};

use super::switch_view;

pub fn build() -> Scenario {
    Scenario::builder("collider_gen_composite")
        .description(
            "Verify stamped tile masks merge into fewer composite contours than per-tile authoring",
        )
        .then(switch_view(LabView::Composite))
        .then(Action::WaitFrames(8))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "composite view active",
            |diagnostics| diagnostics.active_view == LabView::Composite,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "composite merge reduces seam count",
            |diagnostics| {
                diagnostics.composite.placed_tiles == 5
                    && diagnostics.composite.per_tile_contours
                        > diagnostics.composite.composite_contours
                    && diagnostics.composite.composite_pieces > 0
            },
        ))
        .then(Action::Screenshot("composite_merge".into()))
        .then(assertions::log_summary("collider_gen_composite"))
        .build()
}
