//! User interface.

mod player_state;

use crate::{Game, GameOptions};
use impact::{egui, engine::Engine};
use impact_dev_ui::{
    CustomElements, UICommandQueue, UserInterface as DevUserInterface,
    option_panels::{option_group, option_panel},
    overlay::{Corner, TextOverlay},
};
use player_state::DisplayedPlayerState;

pub static UI_COMMANDS: UICommandQueue = UICommandQueue::new();

#[derive(Debug)]
pub struct UserInterface {
    dev_ui: DevUserInterface,
}

#[derive(Debug)]
struct GameUserInterface<'a> {
    options: &'a mut GameOptions,
    player_state: Option<DisplayedPlayerState>,
    currently_interactive: bool,
}

impl Game {
    pub(crate) fn run_ui(
        &mut self,
        ctx: &egui::Context,
        input: egui::RawInput,
    ) -> egui::FullOutput {
        let player_state = match DisplayedPlayerState::gather(self) {
            Ok(state) => Some(state),
            Err(error) => {
                log::error!("Failed to get player state for UI: {error}");
                None
            }
        };
        let mut game_ui = GameUserInterface::new(
            &mut self.game_options,
            player_state,
            self.user_interface.dev_ui.config().interactive,
        );

        self.user_interface.run(
            ctx,
            input,
            self.engine.as_ref().unwrap(),
            &UI_COMMANDS,
            &mut game_ui,
        )
    }

    pub(crate) fn setup_ui(&self) {
        self.user_interface.setup(self.engine());
    }
}

impl UserInterface {
    pub fn new(dev_ui: DevUserInterface) -> Self {
        Self { dev_ui }
    }

    fn setup(&self, engine: &Engine) {
        self.dev_ui.setup(engine);
    }

    fn run(
        &mut self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
        command_queue: &UICommandQueue,
        game_ui: &mut GameUserInterface<'_>,
    ) -> egui::FullOutput {
        self.dev_ui
            .run_with_custom_elements(ctx, input, engine, command_queue, game_ui)
    }
}

impl<'a> GameUserInterface<'a> {
    fn new(
        options: &'a mut GameOptions,
        player_state: Option<DisplayedPlayerState>,
        currently_interactive: bool,
    ) -> Self {
        Self {
            options,
            player_state,
            currently_interactive,
        }
    }

    fn run_game_options(&mut self, ui: &mut egui::Ui) {
        let hot_reloading_enabled = cfg!(feature = "hot_reloading");

        ui.add_enabled(
            hot_reloading_enabled,
            egui::Checkbox::new(
                &mut self.options.reset_scene_on_reload,
                "Reset scene on script reload",
            ),
        )
        .on_hover_text(
            "Whether to reset the scene to the initial state when the script is hot-reloaded.",
        )
        .on_disabled_hover_text("Build with the \"hot_reloading\" feature to enable.");

        ui.end_row();

        if ui.button("Reset scene").clicked() {
            self.options.scene_reset_requested = true;
        }
        ui.end_row();
    }
}

impl<'a> CustomElements for GameUserInterface<'a> {
    fn run_toolbar_buttons(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.options.show_game_options, "Game options");
    }

    fn run_option_panels(&mut self, ctx: &egui::Context, alpha: f32) {
        if !self.options.show_game_options {
            return;
        }
        option_panel(ctx, "game_option_panel", alpha, |ui| {
            option_group(ui, "game_options", |ui| {
                self.run_game_options(ui);
            });
        });
    }

    fn run_overlays(&mut self, ctx: &egui::Context) {
        if self.currently_interactive {
            return;
        }
        let Some(player_state) = &self.player_state else {
            return;
        };

        let alignment_direction_text = format!("Alignment: {}\n", player_state.alignment_direction);

        let acceleration_x_text =
            format!("Acceleration x: {:.1} m/s²", player_state.acceleration.x());
        let acceleration_y_text =
            format!("Acceleration y: {:.1} m/s²", player_state.acceleration.y());
        let acceleration_z_text = format!(
            "Acceleration z: {:.1} m/s²\n",
            player_state.acceleration.z()
        );

        let inventory_mass_text = format!("Inventory: {:.1} kg\n", player_state.inventory_mass);

        let launch_speed_text = format!("Launch speed: {:.1} m/s", player_state.launch_speed);

        TextOverlay::new(egui::Id::new("game_overlay"))
            .corner(Corner::BottomLeft)
            .offset(egui::vec2(10.0, 10.0))
            .show_lines(
                ctx,
                &[
                    &alignment_direction_text,
                    &acceleration_x_text,
                    &acceleration_y_text,
                    &acceleration_z_text,
                    &inventory_mass_text,
                    &launch_speed_text,
                ],
            );
    }
}
