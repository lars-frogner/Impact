use super::{
    option_checkbox, option_group, option_panel_options, option_slider, scientific_formatter,
};
use impact::{
    egui::{Slider, Ui},
    engine::Engine,
};

mod simulation {
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const SPEED_MULTIPLIER: RangeInclusive<f64> = 0.0..=1000.0;
        pub const TIME_STEP_DURATION: RangeInclusive<f64> = 0.0..=1000.0;
        pub const N_SUBSTEPS: RangeInclusive<u32> = 1..=64;
    }
}

mod constraint_solving {
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const N_ITERATIONS: RangeInclusive<u32> = 0..=100;
        pub const OLD_IMPULSE_WEIGHT: RangeInclusive<f64> = 0.0..=1.0;
        pub const N_POSITIONAL_CORRECTION_ITERATIONS: RangeInclusive<u32> = 0..=100;
        pub const POSITIONAL_CORRECTION_FACTOR: RangeInclusive<f64> = 0.0..=1.0;
    }
}

pub(super) fn physics_option_panel(ui: &mut Ui, engine: &Engine) {
    option_panel_options(ui, |ui| {
        option_group(ui, "simulation_options", |ui| {
            simulation_options(ui, engine);
        });
        option_group(ui, "constraint_solving_options", |ui| {
            constraint_solving_options(ui, engine);
        });
    });
}

fn simulation_options(ui: &mut Ui, engine: &Engine) {
    let mut running = engine.simulation_running();
    if option_checkbox(ui, &mut running, "Simulating").changed() {
        engine.set_simulation_running(running);
    }

    let mut simulator = engine.simulator().write().unwrap();

    let matches_frame_duration = simulator.matches_frame_duration_mut();
    option_checkbox(ui, matches_frame_duration, "Real-time");

    if *matches_frame_duration {
        option_slider(
            ui,
            "Speed multiplier",
            Slider::new(
                simulator.simulation_speed_multiplier_mut(),
                simulation::ranges::SPEED_MULTIPLIER,
            )
            .logarithmic(true)
            .suffix("x"),
        );
    } else {
        *simulator.simulation_speed_multiplier_mut() = 1.0;
        option_slider(
            ui,
            "Time step",
            Slider::new(
                simulator.time_step_duration_mut(),
                simulation::ranges::TIME_STEP_DURATION,
            )
            .logarithmic(true)
            .suffix(" s")
            .custom_formatter(scientific_formatter),
        );
    }

    option_slider(
        ui,
        "Substeps",
        Slider::new(simulator.n_substeps_mut(), simulation::ranges::N_SUBSTEPS),
    );
}

fn constraint_solving_options(ui: &mut Ui, engine: &Engine) {
    let simulator = engine.simulator().read().unwrap();
    let mut constraint_manager = simulator.constraint_manager().write().unwrap();
    let constraint_solver = constraint_manager.solver_mut();
    let config = constraint_solver.config_mut();

    option_checkbox(ui, &mut config.enabled, "Constraint solver");

    option_slider(
        ui,
        "Velocity iterations",
        Slider::new(
            &mut config.n_iterations,
            constraint_solving::ranges::N_ITERATIONS,
        ),
    );
    option_slider(
        ui,
        "Warm starting weight",
        Slider::new(
            &mut config.old_impulse_weight,
            constraint_solving::ranges::OLD_IMPULSE_WEIGHT,
        ),
    );
    option_slider(
        ui,
        "Position iterations",
        Slider::new(
            &mut config.n_positional_correction_iterations,
            constraint_solving::ranges::N_POSITIONAL_CORRECTION_ITERATIONS,
        ),
    );
    option_slider(
        ui,
        "Position correction",
        Slider::new(
            &mut config.positional_correction_factor,
            constraint_solving::ranges::POSITIONAL_CORRECTION_FACTOR,
        ),
    );
}
