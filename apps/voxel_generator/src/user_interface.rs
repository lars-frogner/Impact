//! User interface.

use crate::{App, editor::Editor};
use impact::{egui, engine::Engine};
use impact_dev_ui::{UICommandQueue, UserInterface as DevUserInterface};

pub static UI_COMMANDS: UICommandQueue = UICommandQueue::new();

#[derive(Debug)]
pub(crate) struct UserInterface {
    editor: Editor,
    dev_ui: DevUserInterface,
}

impl App {
    pub(crate) fn run_ui(
        &mut self,
        ctx: &egui::Context,
        input: egui::RawInput,
    ) -> egui::FullOutput {
        self.user_interface
            .run(ctx, input, self.engine.as_ref().unwrap(), &UI_COMMANDS)
    }

    pub(crate) fn setup_ui(&self) {
        self.user_interface.setup(self.engine());
    }
}

impl UserInterface {
    pub(crate) fn new(editor: Editor, dev_ui: DevUserInterface) -> Self {
        Self { editor, dev_ui }
    }

    pub(crate) fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
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
    ) -> egui::FullOutput {
        self.dev_ui
            .run_with_custom_elements(ctx, input, engine, command_queue, &mut self.editor)
    }
}
