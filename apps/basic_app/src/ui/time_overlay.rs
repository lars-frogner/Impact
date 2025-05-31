use impact::{
    egui::{Align2, Area, Context, Id, Pos2, TextStyle, vec2},
    engine::Engine,
    game_loop::GameLoop,
};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TimeOverlay;

const OFFSET_FROM_CORNER: [f32; 2] = [-10.0, 6.0];

impl TimeOverlay {
    pub(super) fn run(&mut self, ctx: &Context, game_loop: &GameLoop, engine: &Engine) {
        let simulation_time = engine.simulator().read().unwrap().current_simulation_time();
        let fps = game_loop.smooth_fps();

        let font_id = TextStyle::Body.resolve(&ctx.style());

        let label1 = format!("{simulation_time:.1} s");
        let label2 = format!("{fps:.0} FPS");

        let (galley1, galley2) = ctx.fonts(|f| {
            (
                f.layout_no_wrap(label1.clone(), font_id.clone(), Default::default()),
                f.layout_no_wrap(label2.clone(), font_id.clone(), Default::default()),
            )
        });

        let spacing = 2.0;
        let total_height = galley1.rect.height() + galley2.rect.height() + spacing;
        let max_width = galley1.rect.width().max(galley2.rect.width());

        Area::new(Id::new("time_overlay"))
            .anchor(Align2::RIGHT_TOP, OFFSET_FROM_CORNER) // Top right of screen, with padding
            .interactable(false)
            .show(ctx, |ui| {
                let right = ui.max_rect().right();
                let top = ui.max_rect().top();

                // Draw each label right-aligned
                ui.painter().text(
                    Pos2::new(right, top),
                    Align2::RIGHT_TOP,
                    &label1,
                    font_id.clone(),
                    ui.visuals().text_color(),
                );
                ui.painter().text(
                    Pos2::new(right, top + galley1.rect.height() + spacing),
                    Align2::RIGHT_TOP,
                    &label2,
                    font_id.clone(),
                    ui.visuals().text_color(),
                );

                // Reserve space so nothing overlaps
                ui.allocate_space(vec2(max_width, total_height));
            });
    }
}
