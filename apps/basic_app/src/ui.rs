//! User interface.

use impact::{
    egui::{Align, Align2, Area, Color32, ComboBox, Context, Frame, Id, Layout, SidePanel, Ui},
    engine::Engine,
    game_loop::GameLoop,
};

#[derive(Clone, Debug, Default)]
pub struct UserInterface {
    option_view: OptionView,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OptionView {
    #[default]
    Rendering,
    Simulation,
}

impl UserInterface {
    pub fn run(&mut self, ctx: &Context, game_loop: &GameLoop, engine: &Engine) {
        SidePanel::left("option_panel")
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ComboBox::from_id_salt("option_view_combo_box")
                    .width(ui.available_width())
                    .selected_text(format!("{:?}", self.option_view))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.option_view,
                            OptionView::Rendering,
                            "Rendering",
                        );
                        ui.selectable_value(
                            &mut self.option_view,
                            OptionView::Simulation,
                            "Simulation",
                        );
                    });

                match self.option_view {
                    OptionView::Rendering => {
                        rendering_option_panel(ui, engine);
                    }
                    OptionView::Simulation => {
                        simulation_option_panel(ui, engine);
                    }
                }
            });

        Area::new(Id::new("time_counters"))
            .anchor(Align2::RIGHT_TOP, [-10.0, 10.0])
            .show(ctx, |ui| {
                time_counters(ui, game_loop, engine);
            });
    }
}

fn option_panel_options(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    const LEFT_MARGIN: f32 = 8.0;
    const SPACING: f32 = 6.0;

    ui.add_space(SPACING);
    with_left_space(ui, LEFT_MARGIN, |ui| {
        ui.spacing_mut().item_spacing.y = SPACING;
        add_contents(ui);
    });
}

fn rendering_option_panel(ui: &mut Ui, engine: &Engine) {
    option_panel_options(ui, |ui| {
        ui.label("todo");
    });
}

fn shadow_mapping_checkbox(ui: &mut Ui, engine: &Engine) {
    // let mut running = engine.set_shadow_mapping();
    // if ui.checkbox(&mut running, "Shadow mapping").changed() {
    //     engine.set_simulation_running(running);
    // }
}

fn simulation_option_panel(ui: &mut Ui, engine: &Engine) {
    option_panel_options(ui, |ui| {
        simulation_running_checkbox(ui, engine);
    });
}

fn simulation_running_checkbox(ui: &mut Ui, engine: &Engine) {
    let mut running = engine.simulation_running();
    if ui.checkbox(&mut running, "Running").changed() {
        engine.set_simulation_running(running);
    }
}

fn time_counters(ui: &mut Ui, game_loop: &GameLoop, engine: &Engine) {
    let simulation_time = engine.simulator().read().unwrap().current_simulation_time();
    let fps = game_loop.smooth_fps();
    ui.label(format!("{simulation_time:.1} s"));
    ui.label(format!("{fps} FPS"));
}

fn with_left_space(ui: &mut Ui, amount: f32, add_contents: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
        ui.add_space(amount);
        ui.vertical(|ui| {
            add_contents(ui);
        });
    });
}
