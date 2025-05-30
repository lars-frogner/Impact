//! User interface.

mod physics_options;
mod rendering_options;
mod time_counters;

use impact::{
    egui::{
        Align, Color32, ComboBox, Context, CursorIcon, Frame, Grid, Layout, Response, ScrollArea,
        Separator, SidePanel, Slider, Ui, emath::Numeric,
    },
    engine::Engine,
    game_loop::GameLoop,
};
use std::{hash::Hash, ops::RangeInclusive};

#[derive(Clone, Debug, Default)]
pub struct UserInterface {
    option_view: OptionView,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OptionView {
    #[default]
    Rendering,
    Physics,
}

#[derive(Debug)]
struct LabelAndHoverText {
    label: &'static str,
    hover_text: &'static str,
}

const OPTIONS_LEFT_MARGIN: f32 = 6.0;
const OPTIONS_RIGHT_MARGIN: f32 = 6.0;
const OPTIONS_SPACING: f32 = 6.0;

impl UserInterface {
    pub fn run(&mut self, ctx: &Context, game_loop: &GameLoop, engine: &Engine) {
        SidePanel::left("option_panel")
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ComboBox::from_id_salt("option_view_combo_box")
                    .width(ui.available_width())
                    .selected_text(format!("{:?}", self.option_view))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.option_view,
                            OptionView::Rendering,
                            "Rendering",
                        );
                        ui.selectable_value(&mut self.option_view, OptionView::Physics, "Physics");
                    });

                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match self.option_view {
                        OptionView::Rendering => {
                            rendering_options::rendering_option_panel(ui, engine);
                        }
                        OptionView::Physics => {
                            physics_options::physics_option_panel(ui, engine);
                        }
                    });
            });

        time_counters::time_counters(ctx, game_loop, engine);
    }
}

fn option_panel_options(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    ui.add_space(OPTIONS_SPACING);
    add_contents(ui);
}

fn option_group(ui: &mut Ui, name: impl Hash, add_contents: impl FnOnce(&mut Ui)) {
    with_left_space(ui, OPTIONS_LEFT_MARGIN, |ui| {
        ui.spacing_mut().item_spacing.y = OPTIONS_SPACING;
        Grid::new(name).striped(true).show(ui, |ui| {
            add_contents(ui);
        });
    });
    ui.add(Separator::default());
}

fn option_checkbox(ui: &mut Ui, checked: &mut bool, text: LabelAndHoverText) -> Response {
    let response = ui
        .checkbox(checked, text.label)
        .on_hover_text(text.hover_text);
    ui.end_row();
    response
}

fn option_slider(ui: &mut Ui, text: LabelAndHoverText, slider: Slider<'_>) -> Response {
    labeled_option(ui, text, |ui| ui.add(slider))
}

fn labeled_option<R>(
    ui: &mut Ui,
    text: LabelAndHoverText,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.label(text.label)
        .on_hover_cursor(CursorIcon::Help)
        .on_hover_text(text.hover_text);

    let response = ui
        .horizontal(|ui| {
            let response = add_contents(ui);
            // The subtraction is required to visually match left and right margins
            ui.add_space(OPTIONS_RIGHT_MARGIN - 6.0);
            response
        })
        .inner;

    ui.end_row();
    response
}

fn transform_slider_recip<Num: Numeric>(
    untransformed_value: &mut Num,
    transformed_range: RangeInclusive<f64>,
    add_slider: impl FnOnce(Slider<'_>) -> Response,
) {
    transform_slider(
        untransformed_value,
        transformed_range,
        add_slider,
        f64::recip,
        f64::recip,
    );
}

fn transform_slider<Num: Numeric>(
    untransformed_value: &mut Num,
    transformed_range: RangeInclusive<f64>,
    add_slider: impl FnOnce(Slider<'_>) -> Response,
    transform: impl Fn(f64) -> f64,
    untransform: impl Fn(f64) -> f64,
) {
    let mut transformed_value = transform(untransformed_value.to_f64());
    if add_slider(Slider::new(&mut transformed_value, transformed_range)).changed() {
        *untransformed_value = Num::from_f64(untransform(transformed_value));
    };
}

fn with_left_space(ui: &mut Ui, amount: f32, add_contents: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
        ui.add_space(amount);
        ui.vertical(|ui| {
            add_contents(ui);
        });
    });
}

fn scientific_formatter(value: f64, _decimal_range: std::ops::RangeInclusive<usize>) -> String {
    if value == 0.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.2e}")
    }
}
