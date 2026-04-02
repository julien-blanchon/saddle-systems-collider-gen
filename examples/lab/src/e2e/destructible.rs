use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{LabDiagnostics, LabView};

use super::switch_view;

pub fn build() -> Scenario {
    Scenario::builder("collider_gen_destructible")
        .description("Verify dirty-region ECS regeneration reacts to repeated terrain carving")
        .then(switch_view(LabView::Destructible))
        .then(Action::WaitFrames(12))
        .then(Action::Screenshot("destructible_before".into()))
        .then(Action::WaitFrames(96))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "destructible view active",
            |diagnostics| diagnostics.active_view == LabView::Destructible,
        ))
        .then(assertions::resource_satisfies::<LabDiagnostics>(
            "destructible terrain regenerated after blasts",
            |diagnostics| {
                diagnostics.destructible.blasts_applied >= 2
                    && diagnostics.destructible.regenerations >= 2
                    && diagnostics.destructible.current_filled_pixels
                        < diagnostics.destructible.initial_filled_pixels
                    && diagnostics.destructible.current_pieces > 0
            },
        ))
        .then(Action::Screenshot("destructible_after".into()))
        .then(assertions::log_summary("collider_gen_destructible"))
        .build()
}
