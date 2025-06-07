use super::timing_panel;
use crate::UserInterfaceConfig;
use egui_extras::{Column, TableBuilder};
use impact::{
    egui::{Context, TextStyle},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct TaskTimingPanel;

const NUM_LABEL_COL_CHARS: usize = 24;
const NUM_TIMING_COL_CHARS: usize = 8;

impl TaskTimingPanel {
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
            "task_timing_panel",
            default_panel_width,
            |ui| {
                let header_hight = ui.spacing().interact_size.y;

                TableBuilder::new(ui)
                    .id_salt("task_timings")
                    .striped(true)
                    .column(Column::remainder().resizable(true).clip(true))
                    .column(
                        Column::auto()
                            .at_least(timing_col_width)
                            .at_most(timing_col_width),
                    )
                    .header(header_hight, |mut header| {
                        header.col(|ui| {
                            ui.strong("Task");
                        });
                        header.col(|ui| {
                            ui.strong("Time (µs)");
                        });
                    })
                    .body(|mut body| {
                        for (id, duration) in engine.task_timer().take_task_execution_times() {
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(id.string());
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
