use crate::{AppOptions, BasicApp};
use impact::{egui, engine::Engine};
use impact_dev_ui::{
    CustomPanels, UICommandQueue, UserInterface as DevUserInterface, UserInterfaceConfig,
    option_panels::{option_group, option_panel},
};

pub static UI_COMMANDS: UICommandQueue = UICommandQueue::new();

#[derive(Debug)]
pub struct UserInterface {
    dev_ui: DevUserInterface,
}

impl BasicApp {
    pub fn run_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput {
        self.user_interface.write().run(
            ctx,
            input,
            engine,
            &UI_COMMANDS,
            &mut self.app_options.write(),
        )
    }
}

impl UserInterface {
    pub fn new(dev_ui: DevUserInterface) -> Self {
        Self { dev_ui }
    }

    pub fn setup(&self, engine: &Engine) {
        self.dev_ui.setup(engine);
    }

    pub fn run(
        &mut self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
        command_queue: &UICommandQueue,
        app_options: &mut AppOptions,
    ) -> egui::FullOutput {
        self.dev_ui
            .run_with_custom_panels(ctx, input, engine, command_queue, app_options)
    }
}

impl CustomPanels for AppOptions {
    fn run_toolbar_buttons(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.show_app_options, "App options");
    }

    fn run_panels(&mut self, ctx: &egui::Context, config: &UserInterfaceConfig, _engine: &Engine) {
        if !self.show_app_options {
            return;
        }
        option_panel(ctx, "app_option_panel", config.alpha, |ui| {
            option_group(ui, "app_options", |ui| {
                self.run_app_options(ui);
            });
        });
    }
}

impl AppOptions {
    fn run_app_options(&mut self, ui: &mut egui::Ui) {
        let hot_reloading_enabled = cfg!(feature = "hot_reloading");

        ui.add_enabled(
            hot_reloading_enabled,
            egui::Checkbox::new(
                &mut self.reset_scene_on_reload,
                "Reset scene on script reload",
            ),
        )
        .on_hover_text(
            "Whether to reset the scene to the initial state when the script is hot-reloaded.",
        )
        .on_disabled_hover_text("Build with the \"hot_reloading\" feature to enable.");

        ui.end_row();

        if ui.button("Reset scene").clicked() {
            self.scene_reset_requested = true;
        }
        ui.end_row();
    }
}
