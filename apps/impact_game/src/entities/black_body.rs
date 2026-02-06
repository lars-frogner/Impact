//! Entities emitting black body radiation.

use bytemuck::{Pod, Zeroable};
use impact::{
    engine::Engine,
    impact_ecs::{Component, query},
    impact_light::{
        LuminousIntensity, OmnidirectionalEmission, ShadowableOmnidirectionalEmission, photometry,
    },
    impact_math::consts::{f32::PI, physics::f32::STEFAN_BOLTZMANN_CONSTANT},
    impact_scene::SceneGraphModelInstanceNodeHandle,
};
use roc_integration::roc;

#[roc(parents = "Comp")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod, Component)]
pub struct BlackBody {
    pub surface_area: f32,
    pub heat_capacity: f32,
    pub emissivity: f32,
    pub temperature: f32,
}

#[roc]
impl BlackBody {
    #[roc(body = "{ surface_area, heat_capacity, emissivity, temperature }")]
    pub fn new(surface_area: f32, heat_capacity: f32, emissivity: f32, temperature: f32) -> Self {
        Self {
            surface_area,
            heat_capacity,
            emissivity,
            temperature,
        }
    }

    #[roc(body = r#"
    surface_area = 4 * Num.pi * radius * radius
    heat_capacity = specific_heat_capacity * mass
    { surface_area, heat_capacity, emissivity, temperature }
    "#)]
    pub fn sphere(
        radius: f32,
        mass: f32,
        specific_heat_capacity: f32,
        emissivity: f32,
        temperature: f32,
    ) -> Self {
        let surface_area = 4.0 * PI * radius.powi(2);
        let heat_capacity = specific_heat_capacity * mass;
        Self::new(surface_area, heat_capacity, emissivity, temperature)
    }
}

pub fn update_black_bodies(engine: &Engine) {
    if !engine.simulator_config().enabled {
        return;
    }
    let time_step_duration = engine.time_step_duration();

    let world = engine.queryable_world();

    query!(world, |black_body: &mut BlackBody| {
        update_black_body_temperature(black_body, time_step_duration);
    });

    query!(
        world,
        |black_body: &BlackBody,
         model_instance: &SceneGraphModelInstanceNodeHandle,
         emission: &mut OmnidirectionalEmission| {
            update_black_body_material_and_light(
                engine,
                black_body,
                model_instance,
                Some(&mut emission.luminous_intensity),
            );
        }
    );

    query!(
        world,
        |black_body: &BlackBody,
         model_instance: &SceneGraphModelInstanceNodeHandle,
         emission: &mut ShadowableOmnidirectionalEmission| {
            update_black_body_material_and_light(
                engine,
                black_body,
                model_instance,
                Some(&mut emission.luminous_intensity),
            );
        }
    );

    query!(
        world,
        |black_body: &BlackBody, model_instance: &SceneGraphModelInstanceNodeHandle| {
            update_black_body_material_and_light(engine, black_body, model_instance, None);
        },
        ![OmnidirectionalEmission, ShadowableOmnidirectionalEmission]
    );
}

fn update_black_body_temperature(black_body: &mut BlackBody, time_step_duration: f32) {
    // Radiative power = stef_boltz * T^4 * emiss * area
    // Thermal energy = heat_cap * T
    // If rate of energy loss equals radiative power:
    // Cooling rate = stef_boltz * T^4 * emiss * area / heat_cap
    // dT/dt = -k * T^4, where k = stef_boltz * emiss * area / heat_cap
    // Integrating analytically for temperature change gives:
    // T = (T0^-3 + 3 * k * (t - t0))^(-1/3)

    let scale = 3.0 * STEFAN_BOLTZMANN_CONSTANT * black_body.emissivity * black_body.surface_area
        / black_body.heat_capacity;

    black_body.temperature = (black_body.temperature.powi(3).recip() + scale * time_step_duration)
        .cbrt()
        .recip();
}

fn update_black_body_material_and_light(
    engine: &Engine,
    black_body: &BlackBody,
    model_instance: &SceneGraphModelInstanceNodeHandle,
    luminous_intensity: Option<&mut LuminousIntensity>,
) {
    let rgb_luminance = photometry::compute_black_body_luminance(black_body.temperature);
    let total_luminance = photometry::total_luminance_from_rgb(&rgb_luminance);

    if total_luminance <= 1e-8 {
        return;
    }

    let color = rgb_luminance / total_luminance;

    let _ = engine.with_uniform_physical_material_property_values_mut(model_instance.id, |props| {
        props.color = color;
        props.emissive_luminance = total_luminance;
        Ok(())
    });

    if let Some(luminous_intensity) = luminous_intensity {
        *luminous_intensity = rgb_luminance * black_body.surface_area;
    }
}
