//! Player tools.

use crate::{Game, define_lookup_type};
use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact::impact_ecs::world::EntityID;
use roc_integration::roc;

define_lookup_type! {
    variant = SphereAbsorbedVoxelMass { entity_id: EntityID };
    #[roc(parents = "Lookup")]
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Zeroable, Pod)]
    pub struct SphereAbsorbedVoxelMass {
        mass: f32,
    }
}

define_lookup_type! {
    variant = CapsuleAbsorbedVoxelMass { entity_id: EntityID };
    #[roc(parents = "Lookup")]
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Zeroable, Pod)]
    pub struct CapsuleAbsorbedVoxelMass {
        mass: f32,
    }
}

impl SphereAbsorbedVoxelMass {
    pub fn lookup(game: &Game, entity_id: EntityID) -> Result<Self> {
        let engine = game.engine();

        let absorber_id = engine.get_component_copy(entity_id).with_context(|| {
            anyhow!("Failed to get `VoxelAbsorbingSphereID` component for looking up absorbed mass")
        })?;

        engine.with_absorbed_voxels_for_sphere(
            absorber_id,
            |absorbed_voxels_by_type, mass_densities_by_type| {
                let mass = absorbed_voxels_by_type
                    .iter()
                    .zip(mass_densities_by_type)
                    .map(|(voxels, mass_density)| *mass_density * voxels.volume)
                    .sum();
                println!("absorbed: {mass}");

                Ok(Self { mass })
            },
        )
    }
}

impl CapsuleAbsorbedVoxelMass {
    pub fn lookup(game: &Game, entity_id: EntityID) -> Result<Self> {
        let engine = game.engine();

        let absorber_id = engine.get_component_copy(entity_id).with_context(|| {
            anyhow!(
                "Failed to get `VoxelAbsorbingCapsuleID` component for looking up absorbed mass"
            )
        })?;

        engine.with_absorbed_voxels_for_capsule(
            absorber_id,
            |absorbed_voxels_by_type, mass_densities_by_type| {
                let mass = absorbed_voxels_by_type
                    .iter()
                    .zip(mass_densities_by_type)
                    .map(|(voxels, mass_density)| *mass_density * voxels.volume)
                    .sum();

                Ok(Self { mass })
            },
        )
    }
}
