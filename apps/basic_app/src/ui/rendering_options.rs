use super::{
    labeled_option, option_checkbox, option_group, option_panel_options, option_slider,
    scientific_formatter, transform_slider_recip,
};
use impact::{
    egui::{ComboBox, Slider, Ui},
    engine::{Engine, command::ToActiveState},
    gpu::rendering::{
        RenderingSystem,
        postprocessing::{
            capturing::{SensorSensitivity, tone_mapping::ToneMappingMethod},
            render_attachment_visualization::RenderAttachmentVisualizationPasses,
        },
    },
    util::bounds::{Bounds, UpperExclusiveBounds},
};

mod ambient_occlusion {
    pub mod ranges {
        use impact::gpu::rendering::postprocessing::ambient_occlusion::MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT;
        use std::ops::RangeInclusive;

        pub const SAMPLE_COUNT: RangeInclusive<u32> = 1..=MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32;
        pub const SAMPLE_RADIUS: RangeInclusive<f32> = 0.1..=2.0;
        pub const INTENSITY: RangeInclusive<f32> = 0.1..=10.0;
        pub const CONTRAST: RangeInclusive<f32> = 0.1..=2.0;
    }
}

mod temporal_anti_aliasing {
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const CURRENT_FRAME_WEIGHT: RangeInclusive<f32> = 0.0..=1.0;
        pub const VARIANCE_CLIPPING_THRESHOLD: RangeInclusive<f32> = 0.1..=2.0;
    }
}

mod camera {
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const MAX_EXPOSURE: RangeInclusive<f32> = 1e-6..=1e2;
        pub const EV_COMPENSATION: RangeInclusive<f32> = -10.0..=10.0;
        pub const MIN_LUMINANCE: f32 = 1e-1;
        pub const MAX_LUMINANCE: f32 = 1e12;
        pub const LUMINANCE_FRAME_WEIGHT: RangeInclusive<f32> = 0.0..=1.0;
        pub const RELATIVE_APERTURE: RangeInclusive<f32> = 0.1..=10.0;
        pub const SHUTTER_SPEED: RangeInclusive<f64> = 1.0..=8000.0;
        pub const ISO: RangeInclusive<f32> = 1e1..=1e6;
    }

    pub const DEFAULT_EV_COMPENSATION: f32 = 0.0;
    pub const DEFAULT_ISO: f32 = 100.0;
}

mod bloom {
    pub mod ranges {
        use std::{num::NonZeroU32, ops::RangeInclusive};

        pub const N_DOWNSAMPLINGS: RangeInclusive<NonZeroU32> =
            NonZeroU32::new(1).unwrap()..=NonZeroU32::new(16).unwrap();
        pub const BLUR_FILTER_RADIUS: RangeInclusive<f32> = 1e-4..=1e-1;
        pub const BLURRED_LUMINANCE_WEIGHT: RangeInclusive<f32> = 0.0..=1.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExposureMode {
    Automatic,
    Manual,
}

pub(super) fn rendering_option_panel(ui: &mut Ui, engine: &Engine) {
    let mut renderer = engine.renderer().write().unwrap();
    option_panel_options(ui, |ui| {
        option_group(ui, "shadow_mapping_options", |ui| {
            shadow_mapping_options(ui, &mut renderer);
        });
        option_group(ui, "ambient_occlusion_options", |ui| {
            ambient_occlusion_options(ui, &mut renderer);
        });
        option_group(ui, "temporal_anti_aliasing_options", |ui| {
            temporal_anti_aliasing_options(ui, &mut renderer);
        });
        option_group(ui, "camera_options", |ui| {
            camera_options(ui, &mut renderer);
        });
        option_group(ui, "bloom_options", |ui| {
            bloom_options(ui, &mut renderer);
        });
        option_group(ui, "tone_mapping_options", |ui| {
            tone_mapping_options(ui, &mut renderer);
        });
        option_group(ui, "wireframe_options", |ui| {
            wireframe_options(ui, &mut renderer);
        });
        option_group(ui, "render_attachment_options", |ui| {
            render_attachment_options(ui, &mut renderer);
        });
    });
}

fn shadow_mapping_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    option_checkbox(ui, renderer.shadow_mapping_enabled_mut(), "Shadow mapping");
}

fn ambient_occlusion_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();

    let enabled = postprocessor.ambient_occlusion_enabled_mut();

    option_checkbox(ui, enabled, "Ambient occlusion");

    let mut config = postprocessor.ambient_occlusion_config().clone();

    let sample_count = option_slider(
        ui,
        "Sample count",
        Slider::new(
            &mut config.sample_count,
            ambient_occlusion::ranges::SAMPLE_COUNT,
        ),
    );
    let sample_radius = option_slider(
        ui,
        "Sample radius ",
        Slider::new(
            &mut config.sample_radius,
            ambient_occlusion::ranges::SAMPLE_RADIUS,
        ),
    );
    let intensity = option_slider(
        ui,
        "Intensity",
        Slider::new(&mut config.intensity, ambient_occlusion::ranges::INTENSITY),
    );
    let contrast = option_slider(
        ui,
        "Contrast",
        Slider::new(&mut config.contrast, ambient_occlusion::ranges::CONTRAST),
    );

    if sample_count.changed()
        || sample_radius.changed()
        || intensity.changed()
        || contrast.changed()
    {
        let gpu_resource_group_manager = renderer.gpu_resource_group_manager().read().unwrap();

        postprocessor.set_ambient_occlusion_config(
            renderer.graphics_device(),
            &gpu_resource_group_manager,
            config,
        );
    }
}

fn temporal_anti_aliasing_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();

    let enabled = postprocessor.temporal_anti_aliasing_enabled_mut();

    option_checkbox(ui, enabled, "Temporal AA");

    let mut config = postprocessor.temporal_anti_aliasing_config().clone();

    let current_frame_weight = option_slider(
        ui,
        "Current frame weight",
        Slider::new(
            &mut config.current_frame_weight,
            temporal_anti_aliasing::ranges::CURRENT_FRAME_WEIGHT,
        ),
    );

    let variance_clipping_threshold = option_slider(
        ui,
        "Variance clipping",
        Slider::new(
            &mut config.variance_clipping_threshold,
            temporal_anti_aliasing::ranges::VARIANCE_CLIPPING_THRESHOLD,
        ),
    );

    if current_frame_weight.changed() || variance_clipping_threshold.changed() {
        let gpu_resource_group_manager = renderer.gpu_resource_group_manager().read().unwrap();

        postprocessor.set_temporal_anti_aliasing_config(
            renderer.graphics_device(),
            &gpu_resource_group_manager,
            config,
        );
    }
}

fn camera_options(ui: &mut Ui, renderer: &mut RenderingSystem) {
    let mut postprocessor = renderer.postprocessor().write().unwrap();
    let capturing_camera = postprocessor.capturing_camera_mut();
    let settings = capturing_camera.settings_mut();

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
        Slider::new(&mut settings.max_exposure, camera::ranges::MAX_EXPOSURE)
            .logarithmic(true)
            .suffix("/nit")
            .custom_formatter(scientific_formatter),
    );

    match exposure_mode {
        ExposureMode::Automatic => {
            let mut ev_compensation = match settings.sensitivity {
                SensorSensitivity::Auto { ev_compensation } => ev_compensation,
                SensorSensitivity::Manual { .. } => camera::DEFAULT_EV_COMPENSATION,
            };

            option_slider(
                ui,
                "EV compensation",
                Slider::new(&mut ev_compensation, camera::ranges::EV_COMPENSATION).suffix(" stops"),
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
                Slider::new(
                    &mut min_luminance_value,
                    camera::ranges::MIN_LUMINANCE..=max_luminance_value,
                )
                .logarithmic(true)
                .suffix(" nit")
                .custom_formatter(scientific_formatter),
            );

            let max_luminance = option_slider(
                ui,
                "Max luminance",
                Slider::new(
                    &mut max_luminance_value,
                    min_luminance_value..=camera::ranges::MAX_LUMINANCE,
                )
                .logarithmic(true)
                .suffix(" nit")
                .custom_formatter(scientific_formatter),
            );

            let current_frame_weight = option_slider(
                ui,
                "Current frame weight",
                Slider::new(
                    &mut config.current_frame_weight,
                    camera::ranges::LUMINANCE_FRAME_WEIGHT,
                ),
            );

            if min_luminance.changed() || max_luminance.changed() || current_frame_weight.changed()
            {
                config.luminance_bounds = UpperExclusiveBounds::new(
                    min_luminance_value,
                    max_luminance_value.max(min_luminance_value.next_up()),
                );

                let gpu_resource_group_manager =
                    renderer.gpu_resource_group_manager().read().unwrap();

                capturing_camera.set_average_luminance_computation_config(
                    renderer.graphics_device(),
                    &gpu_resource_group_manager,
                    config,
                );
            }
        }
        ExposureMode::Manual => {
            let mut iso = match settings.sensitivity {
                SensorSensitivity::Manual { iso } => iso,
                SensorSensitivity::Auto { .. } => camera::DEFAULT_ISO,
            };

            option_slider(
                ui,
                "Aperture ratio (F-stop)",
                Slider::new(
                    &mut settings.relative_aperture,
                    camera::ranges::RELATIVE_APERTURE,
                ),
            );

            transform_slider_recip(
                &mut settings.shutter_duration,
                camera::ranges::SHUTTER_SPEED,
                |sl| option_slider(ui, "Shutter speed", sl.suffix("/s")),
            );

            option_slider(
                ui,
                "ISO",
                Slider::new(&mut iso, camera::ranges::ISO).logarithmic(true),
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

    let mut config = capturing_camera.bloom_config().clone();

    let n_downsamplings = option_slider(
        ui,
        "Downsamplings",
        Slider::new(&mut config.n_downsamplings, bloom::ranges::N_DOWNSAMPLINGS),
    );

    let blur_filter_radius = option_slider(
        ui,
        "Blur radius",
        Slider::new(
            &mut config.blur_filter_radius,
            bloom::ranges::BLUR_FILTER_RADIUS,
        )
        .logarithmic(true),
    );

    let blurred_luminance_weight = option_slider(
        ui,
        "Blur weight",
        Slider::new(
            &mut config.blurred_luminance_weight,
            bloom::ranges::BLURRED_LUMINANCE_WEIGHT,
        ),
    );

    if n_downsamplings.changed()
        || blur_filter_radius.changed()
        || blurred_luminance_weight.changed()
    {
        let mut shader_manager = renderer.shader_manager().write().unwrap();
        let mut render_attachment_texture_manager = renderer
            .render_attachment_texture_manager()
            .write()
            .unwrap();

        capturing_camera.set_bloom_config(
            renderer.graphics_device(),
            &mut shader_manager,
            &mut render_attachment_texture_manager,
            config,
        );
    }
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
