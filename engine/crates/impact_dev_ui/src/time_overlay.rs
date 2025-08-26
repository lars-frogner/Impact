use impact::{
    egui::{Align2, Area, Context, Id, Pos2, TextStyle, vec2},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TimeOverlay;

const OFFSET_FROM_CORNER: [f32; 2] = [-10.0, 3.0];
const SPACING: f32 = 12.0;

impl TimeOverlay {
    pub(super) fn run(&mut self, ctx: &Context, engine: &Engine) {
        let simulation_time = engine.simulation_time();
        let fps = engine.current_fps();

        let font_id = TextStyle::Body.resolve(&ctx.style());

        let label1 = format!("{simulation_time:.1} s");
        let label2 = format!("{fps:.0} FPS");

        let (galley1, galley2) = ctx.fonts(|f| {
            (
                f.layout_no_wrap(label1.clone(), font_id.clone(), Default::default()),
                f.layout_no_wrap(label2.clone(), font_id.clone(), Default::default()),
            )
        });

        let total_width = galley1.rect.width() + galley2.rect.width() + SPACING;
        let max_height = galley1.rect.height().max(galley2.rect.height());

        Area::new(Id::new("time_overlay"))
            .anchor(Align2::RIGHT_TOP, OFFSET_FROM_CORNER)
            .interactable(false)
            .show(ctx, |ui| {
                let right = ui.max_rect().right();
                let top = ui.max_rect().top();

                ui.painter().text(
                    Pos2::new(right, top),
                    Align2::RIGHT_TOP,
                    &label1,
                    font_id.clone(),
                    ui.visuals().text_color(),
                );
                ui.painter().text(
                    Pos2::new(right - (galley1.rect.width() + SPACING), top),
                    Align2::RIGHT_TOP,
                    &label2,
                    font_id.clone(),
                    ui.visuals().text_color(),
                );

                // Reserve space so nothing overlaps
                ui.allocate_space(vec2(total_width, max_height));
            });
    }
}
