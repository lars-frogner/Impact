use super::timing_panel;
use crate::UserInterfaceConfig;
use egui_extras::{Column, TableBuilder};
use impact::{
    egui::{Context, TextStyle},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderPassTimingPanel;

const NUM_LABEL_COL_CHARS: usize = 24;
const NUM_TIMING_COL_CHARS: usize = 8;

impl RenderPassTimingPanel {
    pub fn run(&mut self, ctx: &Context, config: &UserInterfaceConfig, engine: &Engine) {
        let style = ctx.style();
        let body_font = TextStyle::Body.resolve(&style);
        let mono_font = TextStyle::Monospace.resolve(&style);

        let mono_char_width = ctx.fonts(|fonts| fonts.glyph_width(&mono_font, 'a'));
        let timing_col_width = NUM_TIMING_COL_CHARS as f32 * mono_char_width;

        let row_height = body_font.size + 2.0;

        let default_panel_width = timing_col_width + NUM_LABEL_COL_CHARS as f32 * body_font.size;

        timing_panel(
            ctx,
            config,
            "render_pass_timing_panel",
            default_panel_width,
            |ui| {
                let renderer = engine.renderer().read().unwrap();
                let timestamp_query_manager = renderer.timestamp_query_manager();

                let header_hight = ui.spacing().interact_size.y;

                TableBuilder::new(ui)
                    .id_salt("render_pass_timings")
                    .striped(true)
                    .column(Column::remainder().resizable(true).clip(true))
                    .column(
                        Column::auto()
                            .at_least(timing_col_width)
                            .at_most(timing_col_width),
                    )
                    .header(header_hight, |mut header| {
                        header.col(|ui| {
                            ui.strong("Render pass");
                        });
                        header.col(|ui| {
                            ui.strong("Time (µs)");
                        });
                    })
                    .body(|mut body| {
                        for (tag, duration) in timestamp_query_manager.last_timing_results() {
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(tag.as_ref());
                                });
                                row.col(|ui| {
                                    ui.monospace(format!(
                                        "{:>width$.1}",
                                        1e6 * duration.as_secs_f64(),
                                        width = NUM_TIMING_COL_CHARS
                                    ));
                                });
                            });
                        }
                    });
            },
        );
    }
}
