pub mod physics;
pub mod rendering;

use crate::ui::UserInterfaceConfig;
use impact::egui::{
    Align, Color32, Context, CursorIcon, Frame, Grid, Id, Layout, Margin, Response, ScrollArea,
    Separator, SidePanel, Slider, Ui, ecolor::linear_u8_from_linear_f32, emath::Numeric,
};
use std::{hash::Hash, ops::RangeInclusive};

#[derive(Debug)]
struct LabelAndHoverText {
    label: &'static str,
    hover_text: &'static str,
}

const LEFT_MARGIN: f32 = 8.0;
const RIGHT_MARGIN: f32 = 8.0;
const SPACING: f32 = 6.0;

fn option_panel(
    ctx: &Context,
    config: &UserInterfaceConfig,
    name: impl Into<Id>,
    add_contents: impl FnOnce(&mut Ui),
) {
    let frame = Frame::side_top_panel(&ctx.style());
    let fill = Color32::from_black_alpha(linear_u8_from_linear_f32(config.alpha).max(1));
    let inner_margin = Margin::ZERO;

    SidePanel::left(name)
        .frame(Frame {
            fill,
            inner_margin,
            ..frame
        })
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(SPACING);
                    add_contents(ui);
                });
        });
}

fn option_group(ui: &mut Ui, name: impl Hash, add_contents: impl FnOnce(&mut Ui)) {
    with_left_space(ui, LEFT_MARGIN, |ui| {
        ui.spacing_mut().item_spacing.y = SPACING;
        Grid::new(name).show(ui, |ui| {
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
            ui.add_space(RIGHT_MARGIN - 6.0);
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
