//! Entities emitting black body radiation.

use crate::{Game, define_lookup_type};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact::{
    impact_ecs::{Component, query},
    impact_light::{Luminance, photometry},
};
use roc_integration::roc;

define_lookup_type! {
    variant = BlackBodyLuminance { temperature: f32 };
    #[roc(parents = "Lookup")]
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Zeroable, Pod)]
    pub struct BlackBodyLuminance {
        rgb_luminance: Luminance,
        total_luminance: f32,
    }
}

impl BlackBodyLuminance {
    pub fn lookup(_game: &Game, temperature: f32) -> Result<Self> {
        let rgb_luminance = photometry::compute_black_body_luminance(temperature);
        let total_luminance = photometry::total_luminance_from_rgb(&rgb_luminance);
        Ok(Self {
            rgb_luminance,
            total_luminance,
        })
    }
}
