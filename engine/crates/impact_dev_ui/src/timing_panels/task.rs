use super::timing_panel;
use crate::UserInterfaceConfig;
use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use egui_extras::{Column, TableBuilder};
use impact::{
    egui::{Context, TextStyle, TextWrapMode},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct TaskTimingPanel;

const NUM_LABEL_COL_CHARS: usize = 24;
const NUM_TIMING_COL_CHARS: usize = 8;

impl TaskTimingPanel {
    pub fn run<A>(&mut self, arena: A, ctx: &Context, config: &UserInterfaceConfig, engine: &Engine)
    where
        A: Allocator,
    {
        let style = ctx.style();
        let body_font = TextStyle::Body.resolve(&style);
        let mono_font = TextStyle::Monospace.resolve(&style);

        let mono_char_width = ctx.fonts(|fonts| fonts.glyph_width(&mono_font, '0'));
        let timing_col_width = (NUM_TIMING_COL_CHARS as f32 * mono_char_width) + 2.0;

        let default_panel_width = timing_col_width + NUM_LABEL_COL_CHARS as f32 * body_font.size;

        timing_panel(
            ctx,
            config,
            "task_timing_panel",
            default_panel_width,
            |ui| {
                let mut task_execution_times = AVec::new_in(arena);
                engine.collect_task_execution_times(&mut task_execution_times);

                let header_height = ui.spacing().interact_size.y;

                let text_h_body = ui.text_style_height(&TextStyle::Body);
                let text_h_mono = ui.text_style_height(&TextStyle::Monospace);

                let row_height = text_h_body.max(text_h_mono) + ui.spacing().item_spacing.y;

                TableBuilder::new(ui)
                    .id_salt("task_timings")
                    .striped(true)
                    .column(Column::remainder().resizable(true).clip(true))
                    .column(
                        Column::auto()
                            .at_least(timing_col_width)
                            .at_most(timing_col_width),
                    )
                    .header(header_height, |mut header| {
                        header.col(|ui| {
                            ui.strong("Task");
                        });
                        header.col(|ui| {
                            ui.strong("Time (µs)");
                        });
                    })
                    .body(|mut body| {
                        for (id, duration) in &task_execution_times {
                            body.row(row_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(id.string());
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
