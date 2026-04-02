use bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{LabDiagnostics, LabView};

use super::switch_view;

pub fn build() -> Scenario {
    Scenario::builder("collider_gen_thresholds")
        .description("Compare alpha, luma, and color-key extraction side by side")
        .then(switch_view(LabView::Thresholds))
        .then(Action::WaitFrames(8))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "threshold view active",
            |diagnostics| diagnostics.active_view == LabView::Thresholds,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "threshold configs diverge",
            |diagnostics| {
                diagnostics.thresholds.alpha_filled > diagnostics.thresholds.keyed_filled
                    && diagnostics.thresholds.luma_filled > 0
                    && diagnostics.thresholds.alpha_filled != diagnostics.thresholds.luma_filled
            },
        ))
        .then(Action::Screenshot("thresholds".into()))
        .then(assertions::log_summary("collider_gen_thresholds"))
        .build()
}
