//! User interface.

use impact::{
    egui::{
        Align, Align2, Area, Color32, ComboBox, Context, Frame, Grid, Id, Layout, Response,
        Separator, SidePanel, Slider, TextStyle, Ui, WidgetText, emath::Numeric,
    },
    engine::{Engine, command::ToActiveState},
    game_loop::GameLoop,
    gpu::rendering::{
        RenderingSystem,
        postprocessing::{
            ambient_occlusion::MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
            capturing::{SensorSensitivity, tone_mapping::ToneMappingMethod},
            render_attachment_visualization::RenderAttachmentVisualizationPasses,
        },
    },
    util::bounds::{Bounds, UpperExclusiveBounds},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExposureMode {
    Automatic,
    Manual,
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

                match self.option_view {
                    OptionView::Rendering => {
                        rendering_option_panel(ui, engine);
                    }
                    OptionView::Physics => {
                        simulation_option_panel(ui, engine);
                    }
                }
            });

        Area::new(Id::new("time_counters"))
            .anchor(Align2::RIGHT_TOP, [-10.0, 6.0])
            .show(ctx, |ui| {
                time_counters(ui, game_loop, engine);
            });
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
    option_separator(ui);
}

fn rendering_option_panel(ui: &mut Ui, engine: &Engine) {
    let mut renderer = engine.renderer().write().unwrap();
    option_panel_options(ui, |ui| {
        option_group(ui, "shadow_mapping", |ui| {
            shadow_mapping_options(ui, &mut renderer);
        });
        option_group(ui, "ambient_occlusion", |ui| {
            ambient_occlusion_options(ui, &mut renderer);
        });
        option_group(ui, "temporal_anti_aliasing", |ui| {
            temporal_anti_aliasing_options(ui, &mut renderer);
        });
        option_group(ui, "camera_settings", |ui| {
            camera_settings(ui, &mut renderer);
        });
        option_group(ui, "bloom", |ui| {
            bloom_options(ui, &mut renderer);
        });
        option_group(ui, "tone_mapping", |ui| {
            tone_mapping_options(ui, &mut renderer);
        });
        option_group(ui, "wireframe", |ui| {
            wireframe_options(ui, &mut renderer);
        });
        option_group(ui, "render_attachment", |ui| {
            render_attachment_options(ui, &mut renderer);
        });
    });
}

fn option_separator(ui: &mut Ui) {
    ui.add(Separator::default());
}

fn shadow_mapping_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    option_checkbox(ui, renderer.shadow_mapping_enabled_mut(), "Shadow mapping");
}

fn ambient_occlusion_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();
    let graphics_device = renderer.graphics_device();
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().read().unwrap();

    let enabled = postprocessor.ambient_occlusion_enabled_mut();

    option_checkbox(ui, enabled, "Ambient occlusion");

    let mut config = postprocessor.ambient_occlusion_config().clone();

    let sample_count = option_slider(
        ui,
        "Sample count",
        Slider::new(
            &mut config.sample_count,
            1..=MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32,
        ),
    );

    let sample_radius = option_slider(
        ui,
        "Sample radius ",
        Slider::new(&mut config.sample_radius, 0.1..=2.0),
    );

    let intensity = option_slider(
        ui,
        "Intensity",
        Slider::new(&mut config.intensity, 0.1..=10.0),
    );

    let contrast = option_slider(ui, "Contrast", Slider::new(&mut config.contrast, 0.1..=2.0));

    if sample_count.changed()
        || sample_radius.changed()
        || intensity.changed()
        || contrast.changed()
    {
        postprocessor.set_ambient_occlusion_config(
            graphics_device,
            &gpu_resource_group_manager,
            config,
        );
    }
}

fn temporal_anti_aliasing_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();
    let graphics_device = renderer.graphics_device();
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().read().unwrap();

    let enabled = postprocessor.temporal_anti_aliasing_enabled_mut();

    option_checkbox(ui, enabled, "Temporal AA");

    let mut config = postprocessor.temporal_anti_aliasing_config().clone();

    let current_frame_weight = option_slider(
        ui,
        "Current frame weight",
        Slider::new(&mut config.current_frame_weight, 0.0..=1.0),
    );

    let variance_clipping_threshold = option_slider(
        ui,
        "Variance clipping",
        Slider::new(&mut config.variance_clipping_threshold, 0.1..=2.0),
    );

    if current_frame_weight.changed() || variance_clipping_threshold.changed() {
        postprocessor.set_temporal_anti_aliasing_config(
            graphics_device,
            &gpu_resource_group_manager,
            config,
        );
    }
}

fn camera_settings(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();
    let capturing_camera = postprocessor.capturing_camera_mut();
    let settings = capturing_camera.settings_mut();

    let graphics_device = renderer.graphics_device();
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().read().unwrap();

    let mut exposure_mode = if settings.sensitivity.is_auto() {
        ExposureMode::Automatic
    } else {
        ExposureMode::Manual
    };

    labeled_option(ui, "Camera exposure", |ui| {
        ComboBox::from_id_salt("Camera exposure")
            .selected_text(format!("{:?}", exposure_mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut exposure_mode, ExposureMode::Automatic, "Automatic");
                ui.selectable_value(&mut exposure_mode, ExposureMode::Manual, "Manual");
            })
    });

    option_slider(
        ui,
        "Max exposure",
        Slider::new(&mut settings.max_exposure, 1e-6..=1e2)
            .logarithmic(true)
            .suffix("/nit")
            .custom_formatter(scientific_formatter),
    );

    match exposure_mode {
        ExposureMode::Automatic => {
            let mut ev_compensation = match settings.sensitivity {
                SensorSensitivity::Auto { ev_compensation } => ev_compensation,
                SensorSensitivity::Manual { .. } => 0.0,
            };

            option_slider(
                ui,
                "EV compensation",
                Slider::new(&mut ev_compensation, -10.0..=10.0).suffix(" stops"),
            );

            settings.sensitivity = SensorSensitivity::Auto { ev_compensation };

            let mut config = capturing_camera
                .average_luminance_computation_config()
                .clone();
            let mut min_luminance_value = config.luminance_bounds.lower();
            let mut max_luminance_value = config.luminance_bounds.upper();

            let min_luminance = option_slider(
                ui,
                "Min luminance",
                Slider::new(&mut min_luminance_value, 1e-1..=max_luminance_value)
                    .logarithmic(true)
                    .suffix(" nit")
                    .custom_formatter(scientific_formatter),
            );

            let max_luminance = option_slider(
                ui,
                "Max luminance",
                Slider::new(&mut max_luminance_value, min_luminance_value..=1e12)
                    .logarithmic(true)
                    .suffix(" nit")
                    .custom_formatter(scientific_formatter),
            );

            let current_frame_weight = option_slider(
                ui,
                "Current frame weight",
                Slider::new(&mut config.current_frame_weight, 0.0..=1.0),
            );

            if min_luminance.changed() || max_luminance.changed() || current_frame_weight.changed()
            {
                config.luminance_bounds = UpperExclusiveBounds::new(
                    min_luminance_value,
                    max_luminance_value.max(min_luminance_value.next_up()),
                );

                capturing_camera.set_average_luminance_computation_config(
                    graphics_device,
                    &gpu_resource_group_manager,
                    config,
                );
            }
        }
        ExposureMode::Manual => {
            let mut iso = match settings.sensitivity {
                SensorSensitivity::Manual { iso } => iso,
                SensorSensitivity::Auto { .. } => 100.0,
            };

            option_slider(
                ui,
                "Aperture ratio (F-stop)",
                Slider::new(&mut settings.relative_aperture, 0.1..=10.0),
            );

            transform_slider_recip(&mut settings.shutter_duration, 1.0..=8000.0, |sl| {
                option_slider(ui, "Shutter speed", sl.suffix(" 1/s"))
            });

            option_slider(
                ui,
                "ISO",
                Slider::new(&mut iso, 1e1..=1e6).logarithmic(true),
            );

            settings.sensitivity = SensorSensitivity::Manual { iso };
        }
    }
}

fn bloom_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();
    let capturing_camera = postprocessor.capturing_camera_mut();

    let enabled = capturing_camera.produces_bloom_mut();

    option_checkbox(ui, enabled, "Bloom");
}

fn tone_mapping_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();
    let capturing_camera = postprocessor.capturing_camera_mut();
    let config = capturing_camera.tone_mapping_config_mut();

    labeled_option(ui, "Tone mapping", |ui| {
        ComboBox::from_id_salt("Tone mapping")
            .selected_text(format!("{:?}", config.method))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut config.method, ToneMappingMethod::ACES, "ACES");
                ui.selectable_value(
                    &mut config.method,
                    ToneMappingMethod::KhronosPBRNeutral,
                    "KhronosPBRNeutral",
                );
                ui.selectable_value(&mut config.method, ToneMappingMethod::None, "None");
            })
    });
}

fn wireframe_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut enabled = renderer.basic_config().wireframe_mode_on;
    if option_checkbox(ui, &mut enabled, "Wireframe mode").changed() {
        renderer.set_wireframe_mode(ToActiveState::from_enabled(enabled));
    }
}

fn render_attachment_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();

    let mut quantity = postprocessor.visualized_render_attachment_quantity();
    let original_quantity = quantity;

    labeled_option(ui, "Render attachment", |ui| {
        ComboBox::from_id_salt("Render attachment")
            .selected_text(if let Some(quantity) = quantity {
                format!("{quantity:?}")
            } else {
                String::from("None")
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut quantity, None, "None");
                for selected_value in RenderAttachmentVisualizationPasses::SUPPORTED_QUANTITIES {
                    ui.selectable_value(
                        &mut quantity,
                        Some(selected_value),
                        format!("{selected_value:?}"),
                    );
                }
            })
    });

    if quantity != original_quantity {
        postprocessor
            .visualize_render_attachment_quantity(quantity)
            .unwrap();
    }
}

fn simulation_option_panel(ui: &mut Ui, engine: &Engine) {
    option_panel_options(ui, |ui| {
        option_group(ui, "simulator", |ui| {
            simulator_options(ui, engine);
        });
        option_group(ui, "constraint_solver", |ui| {
            constraint_solver_options(ui, engine);
        });
    });
}

fn simulator_options(ui: &mut Ui, engine: &Engine) {
    let mut running = engine.simulation_running();
    if option_checkbox(ui, &mut running, "Simulating").changed() {
        engine.set_simulation_running(running);
    }

    let mut simulator = engine.simulator().write().unwrap();

    let matches_frame_duration = simulator.matches_frame_duration_mut();
    option_checkbox(ui, matches_frame_duration, "Real-time");

    if *matches_frame_duration {
        option_slider(
            ui,
            "Speed multiplier",
            Slider::new(simulator.simulation_speed_multiplier_mut(), 0.0..=1000.0)
                .logarithmic(true)
                .suffix("x"),
        );
    } else {
        *simulator.simulation_speed_multiplier_mut() = 1.0;
        option_slider(
            ui,
            "Time step",
            Slider::new(simulator.time_step_duration_mut(), 0.0..=1000.0)
                .logarithmic(true)
                .suffix(" s")
                .custom_formatter(scientific_formatter),
        );
    }

    option_slider(
        ui,
        "Substeps",
        Slider::new(simulator.n_substeps_mut(), 1..=64),
    );
}

fn constraint_solver_options(ui: &mut Ui, engine: &Engine) {
    let simulator = engine.simulator().read().unwrap();
    let mut constraint_manager = simulator.constraint_manager().write().unwrap();
    let constraint_solver = constraint_manager.solver_mut();
    let config = constraint_solver.config_mut();

    option_checkbox(ui, &mut config.enabled, "Constraint solver");

    option_slider(
        ui,
        "Velocity iterations",
        Slider::new(&mut config.n_iterations, 0..=100),
    );
    option_slider(
        ui,
        "Warm starting weight",
        Slider::new(&mut config.old_impulse_weight, 0.0..=1.0),
    );
    option_slider(
        ui,
        "Position iterations",
        Slider::new(&mut config.n_positional_correction_iterations, 0..=100),
    );
    option_slider(
        ui,
        "Position correction",
        Slider::new(&mut config.positional_correction_factor, 0.0..=1.0),
    );
}

fn time_counters(ui: &mut Ui, game_loop: &GameLoop, engine: &Engine) {
    let simulation_time = engine.simulator().read().unwrap().current_simulation_time();
    let fps = game_loop.smooth_fps();
    right_aligned_label(ui, format!("{simulation_time:.1} s"));
    right_aligned_label(ui, format!("{fps:.0} FPS"));
}

fn option_checkbox(ui: &mut Ui, checked: &mut bool, text: impl Into<WidgetText>) -> Response {
    let response = ui.checkbox(checked, text);
    ui.end_row();
    response
}

fn option_slider(ui: &mut Ui, text: impl Into<WidgetText>, slider: Slider<'_>) -> Response {
    labeled_option(ui, text, |ui| ui.add(slider))
}

fn labeled_option<R>(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.label(text);

    let response = ui
        .horizontal(|ui| {
            let response = add_contents(ui);
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

fn right_aligned_label(ui: &mut Ui, text: String) {
    let text_width = ui
        .fonts(|f| {
            f.layout_no_wrap(
                text.clone(),
                TextStyle::Body.resolve(ui.style()),
                ui.visuals().text_color(),
            )
        })
        .rect
        .width();

    let total_width = ui.available_width();
    let left_padding = (total_width - text_width).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(left_padding);
        ui.label(text);
    });
}

fn scientific_formatter(value: f64, _decimal_range: std::ops::RangeInclusive<usize>) -> String {
    if value == 0.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.2e}")
    }
}
