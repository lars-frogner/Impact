use super::timing_panel;
use crate::UserInterfaceConfig;
use egui_extras::{Column, TableBuilder};
use impact::{
    egui::{Context, TextStyle, TextWrapMode},
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

        let mono_char_width = ctx.fonts(|fonts| fonts.glyph_width(&mono_font, '0'));
        let timing_col_width = (NUM_TIMING_COL_CHARS as f32 * mono_char_width) + 2.0;

        let default_panel_width = timing_col_width + NUM_LABEL_COL_CHARS as f32 * body_font.size;

        timing_panel(
            ctx,
            config,
            "render_pass_timing_panel",
            default_panel_width,
            |ui| {
                let timing_results = engine.render_pass_timing_results();

                let header_height = ui.spacing().interact_size.y;

                let text_h_body = ui.text_style_height(&TextStyle::Body);
                let text_h_mono = ui.text_style_height(&TextStyle::Monospace);

                let row_height = text_h_body.max(text_h_mono) + ui.spacing().item_spacing.y;

                TableBuilder::new(ui)
                    .id_salt("render_pass_timings")
                    .striped(true)
                    .column(Column::remainder().resizable(true).clip(true))
                    .column(
                        Column::auto()
                            .at_least(timing_col_width)
                            .at_most(timing_col_width),
                    )
                    .header(header_height, |mut header| {
                        header.col(|ui| {
                            ui.strong("Render pass");
                        });
                        header.col(|ui| {
                            ui.strong("Time (µs)");
                        });
                    })
                    .body(|mut body| {
                        for (tag, duration) in timing_results {
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(tag);
                                });
                                row.col(|ui| {
                                    ui.scope(|ui| {
                                        // Set wrap mode to Extend to prevent
                                        // slight overflow from misaligning the
                                        // columns
                                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                        ui.monospace(format!(
                                            "{:>width$.1}",
                                            1e6 * duration.as_secs_f64(),
                                            width = NUM_TIMING_COL_CHARS
                                        ));
                                    });
                                });
                            });
                        }
                    });
            },
        );
    }
}
