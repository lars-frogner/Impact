use super::{
    labeled_option, option_checkbox, option_group, option_panel, option_slider,
    scientific_formatter, transform_slider_recip,
};
use crate::UserInterfaceConfig;
use impact::{
    command::{
        AdminCommand,
        capture::{CaptureAdminCommand, SaveShadowMapsFor},
        rendering::RenderingAdminCommand,
        rendering::postprocessing::ToRenderAttachmentQuantity,
        uils::ToActiveState,
    },
    egui::{ComboBox, Context, Slider, Ui},
    engine::Engine,
};
use impact_math::bounds::{Bounds, UpperExclusiveBounds};
use impact_rendering::postprocessing::{
    capturing::{SensorSensitivity, dynamic_range_compression::ToneMappingMethod},
    render_attachment_visualization::RenderAttachmentVisualizationPasses,
};

mod capture {
    pub mod docs {
        pub const SCREENSHOT: &str = "Capture screenshot";
        pub const SHADOW_CUBEMAP: &str = "Capture shadow cubemaps";
        pub const CASCADED_SHADOW_MAP: &str = "Capture cascaded shadow maps";

        use crate::option_panels::LabelAndHoverText;

        pub const HIDE_UI_DURING_SCREENSHOTS: LabelAndHoverText = LabelAndHoverText {
            label: "Hide UI during screenshots",
            hover_text: "When enabled, the UI will be hidden when capturing screenshots.",
        };
    }
}

mod shadow_mapping {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Shadow mapping",
            hover_text: "Whether shadow mapping is enabled.",
        };
    }
}

mod ambient_occlusion {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Ambient occlusion",
            hover_text: "Whether ambient occlusion is enabled.",
        };
        pub const SAMPLE_COUNT: LabelAndHoverText = LabelAndHoverText {
            label: "Sample count",
            hover_text: "The number of samples to use for computing ambient occlusion.",
        };
        pub const SAMPLE_RADIUS: LabelAndHoverText = LabelAndHoverText {
            label: "Sample radius",
            hover_text: "The sampling radius to use when computing ambient occlusion.",
        };
        pub const INTENSITY: LabelAndHoverText = LabelAndHoverText {
            label: "Intensity",
            hover_text: "Factor for scaling the intensity of the ambient occlusion.",
        };
        pub const CONTRAST: LabelAndHoverText = LabelAndHoverText {
            label: "Contrast",
            hover_text: "Factor for scaling the contrast of the ambient occlusion.",
        };
    }
    pub mod ranges {
        use impact_rendering::postprocessing::ambient_occlusion::MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT;
        use std::ops::RangeInclusive;

        pub const SAMPLE_COUNT: RangeInclusive<u32> = 1..=MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT as u32;
        pub const SAMPLE_RADIUS: RangeInclusive<f32> = 0.1..=2.0;
        pub const INTENSITY: RangeInclusive<f32> = 0.1..=10.0;
        pub const CONTRAST: RangeInclusive<f32> = 0.1..=2.0;
    }
}

mod temporal_anti_aliasing {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Temporal AA",
            hover_text: "Whether temporal anti-aliasing is enabled.",
        };
        pub const CURRENT_FRAME_WEIGHT: LabelAndHoverText = LabelAndHoverText {
            label: "Current frame weight",
            hover_text: "\
                How much the luminance of the current frame should be weighted compared \
                to the luminance reprojected from the previous frame.",
        };
        pub const VARIANCE_CLIPPING_THRESHOLD: LabelAndHoverText = LabelAndHoverText {
            label: "Variance clipping",
            hover_text: "\
                The maximum variance allowed between the current and previous frame's \
                luminance when performing temporal blending.",
        };
    }
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const CURRENT_FRAME_WEIGHT: RangeInclusive<f32> = 0.0..=1.0;
        pub const VARIANCE_CLIPPING_THRESHOLD: RangeInclusive<f32> = 0.1..=2.0;
    }
}

mod camera {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const EXPOSURE_MODE: LabelAndHoverText = LabelAndHoverText {
            label: "Camera exposure",
            hover_text: "\
                Whether exposure is determined automatically based on incident \
                luminance or manually from camera settings.",
        };
        pub const MAX_EXPOSURE: LabelAndHoverText = LabelAndHoverText {
            label: "Max exposure",
            hover_text: "\
                The maximum exposure of the camera sensor. This corresponds to the \
                reciprocal of the minimum incident luminance in cd/m² that can saturate \
                the sensor.",
        };
        pub const EV_COMPENSATION: LabelAndHoverText = LabelAndHoverText {
            label: "EV compensation",
            hover_text: "\
                The compensation in stops applied to the exposure value \
                obtained from incident luminance.",
        };
        pub const MIN_LUMINANCE: LabelAndHoverText = LabelAndHoverText {
            label: "Min luminance",
            hover_text: "\
                The minimum luminance value that the histogram used for computing \
                average luminance should include (luminance values below this limit \
                will be clipped).",
        };
        pub const MAX_LUMINANCE: LabelAndHoverText = LabelAndHoverText {
            label: "Max luminance",
            hover_text: "\
                The maximum luminance value that the histogram used for computing \
                average luminance should include (luminance values above this limit \
                will be clipped).",
        };
        pub const CURRENT_FRAME_WEIGHT: LabelAndHoverText = LabelAndHoverText {
            label: "Current frame weight",
            hover_text: "\
                How much the average luminance computed for the current frame will be \
                weighted compared to the average luminance computed for the previous \
                frame. A value of 0.0 reuses the previous luminance without \
                modification, while a value of 1.0 uses the current luminance without \
                any contribution from the previous frame.",
        };
        pub const RELATIVE_APERTURE: LabelAndHoverText = LabelAndHoverText {
            label: "Aperture ratio (F-stop)",
            hover_text: "\
                The relative aperture of the camera, which is the ratio of the focal \
                length to the aperture diameter.",
        };
        pub const SHUTTER_SPEED: LabelAndHoverText = LabelAndHoverText {
            label: "Shutter speed",
            hover_text: "The inverse of the duration the sensor is exposed.",
        };
        pub const ISO: LabelAndHoverText = LabelAndHoverText {
            label: "ISO",
            hover_text: "The ISO speed of the camera sensor.",
        };
    }
    pub mod ranges {
        use std::ops::RangeInclusive;

        pub const MAX_EXPOSURE: RangeInclusive<f32> = 1e-6..=1e2;
        pub const EV_COMPENSATION: RangeInclusive<f32> = -10.0..=10.0;
        pub const MIN_LUMINANCE: f32 = 1e-1;
        pub const MAX_LUMINANCE: f32 = 1e12;
        pub const CURRENT_FRAME_WEIGHT: RangeInclusive<f32> = 0.0..=1.0;
        pub const RELATIVE_APERTURE: RangeInclusive<f32> = 1.0..=10.0;
        pub const SHUTTER_SPEED: RangeInclusive<f64> = 1.0..=8000.0;
        pub const ISO: RangeInclusive<f32> = 1e1..=1e6;
    }

    pub const DEFAULT_EV_COMPENSATION: f32 = 0.0;
    pub const DEFAULT_ISO: f32 = 100.0;
}

mod bloom {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Bloom",
            hover_text: "Whether bloom is enabled.",
        };
        pub const N_DOWNSAMPLINGS: LabelAndHoverText = LabelAndHoverText {
            label: "Downsamplings",
            hover_text: "\
                The number of downsamplings to perform during blurring. More \
                downsamplings will result in stronger blurring.",
        };
        pub const BLUR_FILTER_RADIUS: LabelAndHoverText = LabelAndHoverText {
            label: "Blur radius",
            hover_text: "\
                The radius of the blur filter to apply during upsampling. A larger \
                radius will result in stronger blurring.",
        };
        pub const BLURRED_LUMINANCE_WEIGHT: LabelAndHoverText = LabelAndHoverText {
            label: "Blur weight",
            hover_text: "\
                How strongly the blurred luminance should be weighted when blending with \
                the original luminance. A value of zero will result in no blending, \
                effectively disabling bloom. A value of one will replace the original \
                luminance with the blurred luminance.",
        };
    }
    pub mod ranges {
        use std::{num::NonZeroU32, ops::RangeInclusive};

        pub const N_DOWNSAMPLINGS: RangeInclusive<NonZeroU32> =
            NonZeroU32::new(1).unwrap()..=NonZeroU32::new(16).unwrap();
        pub const BLUR_FILTER_RADIUS: RangeInclusive<f32> = 1e-4..=1e-1;
        pub const BLURRED_LUMINANCE_WEIGHT: RangeInclusive<f32> = 0.0..=1.0;
    }
}

mod dynamic_range_compression {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const TONE_MAPPING_METHOD: LabelAndHoverText = LabelAndHoverText {
            label: "Tone mapping",
            hover_text: "The method to use for tone mapping.",
        };
    }
}

mod wireframe {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const ENABLED: LabelAndHoverText = LabelAndHoverText {
            label: "Wireframe mode",
            hover_text: "Whether only triangle edges instead of faces should be rendered.",
        };
    }
}

mod render_attachment {
    pub mod docs {
        use crate::option_panels::LabelAndHoverText;

        pub const ATTACHMENT: LabelAndHoverText = LabelAndHoverText {
            label: "Render attachment",
            hover_text: "Which specific render attachment texture to visualize.",
        };
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderingOptionPanel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExposureMode {
    Automatic,
    Manual,
}

impl RenderingOptionPanel {
    pub fn run(
        &mut self,
        ctx: &Context,
        config: &mut UserInterfaceConfig,
        engine: &Engine,
        screenshot_requested: &mut bool,
    ) {
        let mut hide_ui_during_screenshots = config.hide_ui_during_screenshots;
        option_panel(ctx, config, "rendering_option_panel", |ui| {
            option_group(ui, "shadow_mapping_options", |ui| {
                shadow_mapping_options(ui, engine);
            });
            option_group(ui, "ambient_occlusion_options", |ui| {
                ambient_occlusion_options(ui, engine);
            });
            option_group(ui, "temporal_anti_aliasing_options", |ui| {
                temporal_anti_aliasing_options(ui, engine);
            });

            option_group(ui, "camera_options", |ui| {
                camera_options(ui, engine);
            });
            option_group(ui, "bloom_options", |ui| {
                bloom_options(ui, engine);
            });
            option_group(ui, "dynamic_range_compression_options", |ui| {
                dynamic_range_compression_options(ui, engine);
            });
            option_group(ui, "wireframe_options", |ui| {
                wireframe_options(ui, engine);
            });
            option_group(ui, "render_attachment_options", |ui| {
                render_attachment_options(ui, engine);
            });
            option_group(ui, "capture_options", |ui| {
                capture_options(
                    ui,
                    engine,
                    screenshot_requested,
                    &mut hide_ui_during_screenshots,
                );
            });
        });
        config.hide_ui_during_screenshots = hide_ui_during_screenshots;
    }
}

fn shadow_mapping_options(ui: &mut Ui, engine: &Engine) {
    let mut enabled = engine.shadow_mapping_enabled();
    if option_checkbox(ui, &mut enabled, shadow_mapping::docs::ENABLED).changed() {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetShadowMapping(ToActiveState::from_enabled(enabled)),
        ));
    }
}

fn ambient_occlusion_options(ui: &mut Ui, engine: &Engine) {
    let mut config = engine.ambient_occlusion_config();
    let mut config_changed = false;

    if option_checkbox(ui, &mut config.enabled, ambient_occlusion::docs::ENABLED).changed() {
        config_changed = true;
    }

    if option_slider(
        ui,
        ambient_occlusion::docs::SAMPLE_COUNT,
        Slider::new(
            &mut config.sample_count,
            ambient_occlusion::ranges::SAMPLE_COUNT,
        ),
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        ambient_occlusion::docs::SAMPLE_RADIUS,
        Slider::new(
            &mut config.sample_radius,
            ambient_occlusion::ranges::SAMPLE_RADIUS,
        ),
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        ambient_occlusion::docs::INTENSITY,
        Slider::new(&mut config.intensity, ambient_occlusion::ranges::INTENSITY),
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        ambient_occlusion::docs::CONTRAST,
        Slider::new(&mut config.contrast, ambient_occlusion::ranges::CONTRAST),
    )
    .changed()
    {
        config_changed = true;
    }

    if config_changed {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetAmbientOcclusionConfig(config),
        ));
    }
}

fn temporal_anti_aliasing_options(ui: &mut Ui, engine: &Engine) {
    let mut config = engine.temporal_anti_aliasing_config();
    let mut config_changed = false;

    if option_checkbox(
        ui,
        &mut config.enabled,
        temporal_anti_aliasing::docs::ENABLED,
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        temporal_anti_aliasing::docs::CURRENT_FRAME_WEIGHT,
        Slider::new(
            &mut config.current_frame_weight,
            temporal_anti_aliasing::ranges::CURRENT_FRAME_WEIGHT,
        ),
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        temporal_anti_aliasing::docs::VARIANCE_CLIPPING_THRESHOLD,
        Slider::new(
            &mut config.variance_clipping_threshold,
            temporal_anti_aliasing::ranges::VARIANCE_CLIPPING_THRESHOLD,
        ),
    )
    .changed()
    {
        config_changed = true;
    }

    if config_changed {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetTemporalAntiAliasingConfig(config),
        ));
    }
}

fn camera_options(ui: &mut Ui, engine: &Engine) {
    let mut settings = engine.camera_settings();
    let mut avg_luminance_config = engine.average_luminance_computation_config();

    let mut exposure_mode = if settings.sensitivity.is_auto() {
        ExposureMode::Automatic
    } else {
        ExposureMode::Manual
    };

    labeled_option(ui, camera::docs::EXPOSURE_MODE, |ui| {
        ComboBox::from_id_salt(camera::docs::EXPOSURE_MODE.label)
            .selected_text(format!("{exposure_mode:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut exposure_mode, ExposureMode::Automatic, "Automatic");
                ui.selectable_value(&mut exposure_mode, ExposureMode::Manual, "Manual");
            })
    });

    let mut settings_changed = false;

    if option_slider(
        ui,
        camera::docs::MAX_EXPOSURE,
        Slider::new(&mut settings.max_exposure, camera::ranges::MAX_EXPOSURE)
            .logarithmic(true)
            .suffix("/nit")
            .custom_formatter(scientific_formatter),
    )
    .changed()
    {
        settings_changed = true;
    }

    match exposure_mode {
        ExposureMode::Automatic => {
            let mut ev_compensation = match settings.sensitivity {
                SensorSensitivity::Auto { ev_compensation } => ev_compensation,
                SensorSensitivity::Manual { .. } => camera::DEFAULT_EV_COMPENSATION,
            };

            if option_slider(
                ui,
                camera::docs::EV_COMPENSATION,
                Slider::new(&mut ev_compensation, camera::ranges::EV_COMPENSATION).suffix(" stops"),
            )
            .changed()
            {
                settings.sensitivity = SensorSensitivity::Auto { ev_compensation };
                settings_changed = true;
            }

            let mut min_luminance_value = avg_luminance_config.luminance_bounds.lower();
            let mut max_luminance_value = avg_luminance_config.luminance_bounds.upper();
            let mut avg_config_changed = false;

            if option_slider(
                ui,
                camera::docs::MIN_LUMINANCE,
                Slider::new(
                    &mut min_luminance_value,
                    camera::ranges::MIN_LUMINANCE..=max_luminance_value,
                )
                .logarithmic(true)
                .suffix(" nit")
                .custom_formatter(scientific_formatter),
            )
            .changed()
            {
                avg_config_changed = true;
            }

            if option_slider(
                ui,
                camera::docs::MAX_LUMINANCE,
                Slider::new(
                    &mut max_luminance_value,
                    min_luminance_value..=camera::ranges::MAX_LUMINANCE,
                )
                .logarithmic(true)
                .suffix(" nit")
                .custom_formatter(scientific_formatter),
            )
            .changed()
            {
                avg_config_changed = true;
            }

            if option_slider(
                ui,
                camera::docs::CURRENT_FRAME_WEIGHT,
                Slider::new(
                    &mut avg_luminance_config.current_frame_weight,
                    camera::ranges::CURRENT_FRAME_WEIGHT,
                ),
            )
            .changed()
            {
                avg_config_changed = true;
            }

            if avg_config_changed {
                avg_luminance_config.luminance_bounds = UpperExclusiveBounds::new(
                    min_luminance_value,
                    max_luminance_value.max(min_luminance_value.next_up()),
                );

                engine.enqueue_admin_command(AdminCommand::Rendering(
                    RenderingAdminCommand::SetAverageLuminanceComputationConfig(
                        avg_luminance_config,
                    ),
                ));
            }
        }
        ExposureMode::Manual => {
            let mut iso = match settings.sensitivity {
                SensorSensitivity::Manual { iso } => iso,
                SensorSensitivity::Auto { .. } => camera::DEFAULT_ISO,
            };

            if option_slider(
                ui,
                camera::docs::RELATIVE_APERTURE,
                Slider::new(
                    &mut settings.relative_aperture,
                    camera::ranges::RELATIVE_APERTURE,
                ),
            )
            .changed()
            {
                settings_changed = true;
            }

            let mut shutter_changed = false;
            transform_slider_recip(
                &mut settings.shutter_duration,
                camera::ranges::SHUTTER_SPEED,
                |sl| {
                    let response = option_slider(ui, camera::docs::SHUTTER_SPEED, sl.suffix("/s"));
                    if response.changed() {
                        shutter_changed = true;
                    }
                    response
                },
            );
            if shutter_changed {
                settings_changed = true;
            }

            if option_slider(
                ui,
                camera::docs::ISO,
                Slider::new(&mut iso, camera::ranges::ISO).logarithmic(true),
            )
            .changed()
            {
                settings.sensitivity = SensorSensitivity::Manual { iso };
                settings_changed = true;
            }
        }
    }

    if settings_changed {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetCameraSettings(settings),
        ));
    }
}

fn bloom_options(ui: &mut Ui, engine: &Engine) {
    let mut config = engine.bloom_config();
    let mut config_changed = false;

    if option_checkbox(ui, &mut config.enabled, bloom::docs::ENABLED).changed() {
        config_changed = true;
    }

    if option_slider(
        ui,
        bloom::docs::N_DOWNSAMPLINGS,
        Slider::new(&mut config.n_downsamplings, bloom::ranges::N_DOWNSAMPLINGS),
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        bloom::docs::BLUR_FILTER_RADIUS,
        Slider::new(
            &mut config.blur_filter_radius,
            bloom::ranges::BLUR_FILTER_RADIUS,
        )
        .logarithmic(true),
    )
    .changed()
    {
        config_changed = true;
    }

    if option_slider(
        ui,
        bloom::docs::BLURRED_LUMINANCE_WEIGHT,
        Slider::new(
            &mut config.blurred_luminance_weight,
            bloom::ranges::BLURRED_LUMINANCE_WEIGHT,
        ),
    )
    .changed()
    {
        config_changed = true;
    }

    if config_changed {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetBloomConfig(config),
        ));
    }
}

fn dynamic_range_compression_options(ui: &mut Ui, engine: &Engine) {
    let mut config = engine.dynamic_range_compression_config();
    let mut config_changed = false;

    labeled_option(
        ui,
        dynamic_range_compression::docs::TONE_MAPPING_METHOD,
        |ui| {
            ComboBox::from_id_salt(dynamic_range_compression::docs::TONE_MAPPING_METHOD.label)
                .selected_text(format!("{:?}", config.tone_mapping_method))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut config.tone_mapping_method,
                            ToneMappingMethod::ACES,
                            "ACES",
                        )
                        .changed()
                    {
                        config_changed = true;
                    }
                    if ui
                        .selectable_value(
                            &mut config.tone_mapping_method,
                            ToneMappingMethod::KhronosPBRNeutral,
                            "KhronosPBRNeutral",
                        )
                        .changed()
                    {
                        config_changed = true;
                    }
                    if ui
                        .selectable_value(
                            &mut config.tone_mapping_method,
                            ToneMappingMethod::None,
                            "None",
                        )
                        .changed()
                    {
                        config_changed = true;
                    }
                })
        },
    );

    if config_changed {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetDynamicRangeCompressionConfig(config),
        ));
    }
}

fn wireframe_options(ui: &mut Ui, engine: &Engine) {
    let mut enabled = engine.basic_rendering_config().wireframe_mode_on;
    if option_checkbox(ui, &mut enabled, wireframe::docs::ENABLED).changed() {
        engine.enqueue_admin_command(AdminCommand::Rendering(
            RenderingAdminCommand::SetWireframeMode(ToActiveState::from_enabled(enabled)),
        ));
    }
}

fn render_attachment_options(ui: &mut Ui, engine: &Engine) {
    let mut quantity = engine.visualized_render_attachment_quantity();
    let original_quantity = quantity;

    labeled_option(ui, render_attachment::docs::ATTACHMENT, |ui| {
        ComboBox::from_id_salt(render_attachment::docs::ATTACHMENT.label)
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
        if let Some(q) = quantity {
            engine.enqueue_admin_command(AdminCommand::Rendering(
                RenderingAdminCommand::SetRenderAttachmentVisualization(ToActiveState::Enabled),
            ));
            engine.enqueue_admin_command(AdminCommand::Rendering(
                RenderingAdminCommand::SetVisualizedRenderAttachmentQuantity(
                    ToRenderAttachmentQuantity::Specific(q),
                ),
            ));
        } else {
            engine.enqueue_admin_command(AdminCommand::Rendering(
                RenderingAdminCommand::SetRenderAttachmentVisualization(ToActiveState::Disabled),
            ));
        }
    }
}

fn capture_options(
    ui: &mut Ui,
    engine: &Engine,
    screenshot_requested: &mut bool,
    hide_ui_during_screenshots: &mut bool,
) {
    option_checkbox(
        ui,
        hide_ui_during_screenshots,
        capture::docs::HIDE_UI_DURING_SCREENSHOTS,
    );
    ui.end_row();

    if ui.button(capture::docs::SCREENSHOT).clicked() {
        engine.enqueue_admin_command(AdminCommand::Capture(CaptureAdminCommand::SaveScreenshot));
        *screenshot_requested = true;
    }
    ui.end_row();

    if ui.button(capture::docs::SHADOW_CUBEMAP).clicked() {
        engine.enqueue_admin_command(AdminCommand::Capture(CaptureAdminCommand::SaveShadowMaps(
            SaveShadowMapsFor::OmnidirectionalLight,
        )));
    }
    ui.end_row();

    if ui.button(capture::docs::CASCADED_SHADOW_MAP).clicked() {
        engine.enqueue_admin_command(AdminCommand::Capture(CaptureAdminCommand::SaveShadowMaps(
            SaveShadowMapsFor::UnidirectionalLight,
        )));
    }
    ui.end_row();
}
