use super::{option_checkbox, option_group, option_panel, option_slider, scientific_formatter};
use crate::ui::UserInterfaceConfig;
use impact::{
    egui::{Context, Slider, Ui},
    engine::Engine,
};

mod simulation {
    pub mod docs {
        use crate::ui::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Simulating",
            hover_text: "Whether the physics simulation is running.",
        };
        pub const REALTIME: LabelAndHoverText = LabelAndHoverText {
            label: "Real-time",
            hover_text: "\
                If enabled, the time step duration will be updated regularly to match the \
                frame duration.",
        };
        pub const SPEED_MULTIPLIER: LabelAndHoverText = LabelAndHoverText {
            label: "Speed multiplier",
            hover_text: "\
                The multiplier applied the base time step duration to \
                change the speed of the simulation.",
        };
        pub const TIME_STEP_DURATION: LabelAndHoverText = LabelAndHoverText {
            label: "Time step",
            hover_text: "The base duration of each simulation time step.",
        };
        pub const N_SUBSTEPS: LabelAndHoverText = LabelAndHoverText {
            label: "Substeps",
            hover_text: "\
                The number of substeps to perform each simulation step. Increase to \
                improve accuracy.",
        };
    }
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const SPEED_MULTIPLIER: RangeInclusive<f64> = 0.0..=1000.0;
        pub const TIME_STEP_DURATION: RangeInclusive<f64> = 0.0..=1000.0;
        pub const N_SUBSTEPS: RangeInclusive<u32> = 1..=64;
    }
}

mod constraint_solving {
    pub mod docs {
        use crate::ui::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Constraint solver",
            hover_text: "Whether constraints will be solved.",
        };
        pub const N_ITERATIONS: LabelAndHoverText = LabelAndHoverText {
            label: "Velocity iterations",
            hover_text: "\
                The number of sequential impulse iterations to perform for solving the \
                velocity constraints.",
        };
        pub const OLD_IMPULSE_WEIGHT: LabelAndHoverText = LabelAndHoverText {
            label: "Warm starting weight",
            hover_text: "\
                How to scale the still-valid accumulated impulses from the previous \
                frame before using them as the initial impulses for the current frame. \
                Set to zero to disable warm starting.",
        };
        pub const N_POSITIONAL_CORRECTION_ITERATIONS: LabelAndHoverText = LabelAndHoverText {
            label: "Position iterations",
            hover_text: "\
                The number of iterations to use for positional correction after the \
                velocity constraints have been solved.",
        };
        pub const POSITIONAL_CORRECTION_FACTOR: LabelAndHoverText = LabelAndHoverText {
            label: "Position correction",
            hover_text: "\
                The fraction of the current positional error the solver should try to \
                correct.",
        };
    }
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const N_ITERATIONS: RangeInclusive<u32> = 0..=100;
        pub const OLD_IMPULSE_WEIGHT: RangeInclusive<f64> = 0.0..=1.0;
        pub const N_POSITIONAL_CORRECTION_ITERATIONS: RangeInclusive<u32> = 0..=100;
        pub const POSITIONAL_CORRECTION_FACTOR: RangeInclusive<f64> = 0.0..=1.0;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicsOptionPanel;

impl PhysicsOptionPanel {
    pub fn run(&mut self, ctx: &Context, config: &UserInterfaceConfig, engine: &Engine) {
        option_panel(ctx, config, "physics_option_panel", |ui| {
            option_group(ui, "simulation_options", |ui| {
                simulation_options(ui, engine);
            });
            option_group(ui, "constraint_solving_options", |ui| {
                constraint_solving_options(ui, engine);
            });
        });
    }
}

fn simulation_options(ui: &mut Ui, engine: &Engine) {
    let mut running = engine.simulation_running();
    if option_checkbox(ui, &mut running, simulation::docs::ENABLED).changed() {
        engine.set_simulation_running(running);
    }

    let mut simulator = engine.simulator().write().unwrap();

    let matches_frame_duration = simulator.matches_frame_duration_mut();
    option_checkbox(ui, matches_frame_duration, simulation::docs::REALTIME);

    if *matches_frame_duration {
        option_slider(
            ui,
            simulation::docs::SPEED_MULTIPLIER,
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
            simulation::docs::TIME_STEP_DURATION,
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
        simulation::docs::N_SUBSTEPS,
        Slider::new(simulator.n_substeps_mut(), simulation::ranges::N_SUBSTEPS),
    );
}

fn constraint_solving_options(ui: &mut Ui, engine: &Engine) {
    let simulator = engine.simulator().read().unwrap();
    let mut constraint_manager = simulator.constraint_manager().write().unwrap();
    let constraint_solver = constraint_manager.solver_mut();
    let config = constraint_solver.config_mut();

    option_checkbox(ui, &mut config.enabled, constraint_solving::docs::ENABLED);

    option_slider(
        ui,
        constraint_solving::docs::N_ITERATIONS,
        Slider::new(
            &mut config.n_iterations,
            constraint_solving::ranges::N_ITERATIONS,
        ),
    );
    option_slider(
        ui,
        constraint_solving::docs::OLD_IMPULSE_WEIGHT,
        Slider::new(
            &mut config.old_impulse_weight,
            constraint_solving::ranges::OLD_IMPULSE_WEIGHT,
        ),
    );
    option_slider(
        ui,
        constraint_solving::docs::N_POSITIONAL_CORRECTION_ITERATIONS,
        Slider::new(
            &mut config.n_positional_correction_iterations,
            constraint_solving::ranges::N_POSITIONAL_CORRECTION_ITERATIONS,
        ),
    );
    option_slider(
        ui,
        constraint_solving::docs::POSITIONAL_CORRECTION_FACTOR,
        Slider::new(
            &mut config.positional_correction_factor,
            constraint_solving::ranges::POSITIONAL_CORRECTION_FACTOR,
        ),
    );
}
