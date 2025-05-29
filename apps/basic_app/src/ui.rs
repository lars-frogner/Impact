//! User interface.

use impact::{egui, engine::Engine};

pub fn run(ctx: &egui::Context, engine: &Engine) {
    egui::CentralPanel::default()
        .frame(egui::Frame {
            fill: egui::Color32::TRANSPARENT,
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label("Impact");
                ui.add_space(10.0);
                if ui.button("Exit").clicked() {
                    engine.request_shutdown();
                }
            })
        });
}
