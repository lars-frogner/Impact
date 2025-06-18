use super::{LabelAndHoverText, option_checkbox, option_group, option_panel, option_slider};
use crate::UserInterfaceConfig;
use impact::{
    egui::{Context, Response, Slider, Ui},
    engine::Engine,
    gizmo::{GizmoManager, GizmoParameters, GizmoType, GizmoVisibility},
};

fn gizmo_parameter_options(
    ui: &mut Ui,
    parameters: &mut GizmoParameters,
    gizmo: GizmoType,
) -> Option<Response> {
    match gizmo {
        GizmoType::CenterOfMass => Some(option_slider(
            ui,
            LabelAndHoverText {
                label: "Center of mass sphere density",
                hover_text: "\
                    The density used to calculate the size of the center \
                    of mass sphere from the mass of the body.",
            },
            Slider::new(
                &mut parameters.center_of_mass_sphere_density,
                1.0..=100000.0,
            )
            .logarithmic(true),
        )),
        GizmoType::LinearVelocity => Some(option_slider(
            ui,
            LabelAndHoverText {
                label: "Linear velocity scale",
                hover_text: "\
                    The scale factor used to calculate the length of the \
                    linear velocity arrow based on the entity's speed.",
            },
            Slider::new(&mut parameters.linear_velocity_scale, 0.0..=10000.0).logarithmic(true),
        )),
        GizmoType::AngularVelocity => Some(option_slider(
            ui,
            LabelAndHoverText {
                label: "Angular velocity scale",
                hover_text: "\
                    The scale factor used to calculate the length of the \
                    angular velocity arrow based on the entity's angular \
                    speed.",
            },
            Slider::new(&mut parameters.angular_velocity_scale, 0.0..=10000.0).logarithmic(true),
        )),
        GizmoType::AngularMomentum => Some(option_slider(
            ui,
            LabelAndHoverText {
                label: "Angular momentum scale",
                hover_text: "\
                    The scale factor used to calculate the length of the \
                    angular momentum arrow based on the magnitude of the \
                    body's angular momentum.",
            },
            Slider::new(&mut parameters.angular_momentum_scale, 0.0..=10000.0).logarithmic(true),
        )),
        GizmoType::Force => Some(option_slider(
            ui,
            LabelAndHoverText {
                label: "Force scale",
                hover_text: "\
                    The scale factor used to calculate the length of the \
                    force arrow based on the magnitude of the force on the \
                    body.",
            },
            Slider::new(&mut parameters.force_scale, 0.0..=10000.0).logarithmic(true),
        )),
        GizmoType::Torque => Some(option_slider(
            ui,
            LabelAndHoverText {
                label: "Torque scale",
                hover_text: "\
                    The scale factor used to calculate the length of the \
                    torque arrow based on the magnitude of the torque on \
                    the body.",
            },
            Slider::new(&mut parameters.torque_scale, 0.0..=10000.0).logarithmic(true),
        )),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GizmoOptionPanel;

impl GizmoOptionPanel {
    pub fn run(&mut self, ctx: &Context, config: &UserInterfaceConfig, engine: &Engine) {
        let mut gizmo_manager = engine.gizmo_manager().write().unwrap();

        option_panel(ctx, config, "gizmo_option_panel", |ui| {
            option_group(ui, "gizmo_options", |ui| {
                gizmo_options(ui, &mut gizmo_manager);
            });
        });
    }
}

fn gizmo_options(ui: &mut Ui, gizmo_manager: &mut GizmoManager) {
    for gizmo in GizmoType::all() {
        let mut visible = gizmo_manager
            .visibilities()
            .get_for(gizmo)
            .is_visible_for_all();

        if option_checkbox(
            ui,
            &mut visible,
            LabelAndHoverText {
                label: gizmo.label(),
                hover_text: gizmo.description(),
            },
        )
        .changed()
        {
            gizmo_manager.set_visibility_for_gizmo(
                gizmo,
                if visible {
                    GizmoVisibility::VisibleForAll
                } else {
                    GizmoVisibility::Hidden
                },
            );
        }

        gizmo_parameter_options(ui, gizmo_manager.parameters_mut(), gizmo);
    }
}
