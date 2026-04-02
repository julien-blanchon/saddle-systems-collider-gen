mod atlas;
mod composite;
mod destructible;
mod smoke;
mod thresholds;

use bevy::prelude::*;
use bevy_e2e::{action::Action, scenario::Scenario};

use crate::{LabView, set_active_view};

pub struct ColliderGenLabE2EPlugin;

impl Plugin for ColliderGenLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_e2e::E2EPlugin);

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                bevy_e2e::init_scenario(app, scenario);
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
        "collider_gen_smoke" => Some(smoke::build()),
        "collider_gen_thresholds" => Some(thresholds::build()),
        "collider_gen_atlas" => Some(atlas::build()),
        "collider_gen_composite" => Some(composite::build()),
        "collider_gen_destructible" => Some(destructible::build()),
        _ => None,
    }
}

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "collider_gen_smoke",
        "collider_gen_thresholds",
        "collider_gen_atlas",
        "collider_gen_composite",
        "collider_gen_destructible",
    ]
}
