//! Input/output of mesh data in Wavefront OBJ format.

use crate::{
    assets::Assets,
    gpu::texture::{ColorSpace, SamplerConfig, TextureAddressingConfig, TextureConfig},
    material::{
        RGBColor,
        components::{
            NormalMapComp, TexturedColorComp, TexturedSpecularReflectanceComp, UniformColorComp,
        },
    },
    mesh::{
        MeshID, MeshRepository, TriangleMesh, VertexNormalVector, VertexPosition,
        VertexTextureCoords, components::MeshComp, texture_projection::TextureProjection,
    },
};
use anyhow::{Result, bail};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentStorage, SingleInstance},
};
use impact_math::hash64;
use nalgebra::{UnitVector3, point, vector};
use std::{
    collections::{HashMap, hash_map::Entry},
    fmt::Debug,
    path::Path,
};
use tobj::{GPU_LOAD_OPTIONS, Material as ObjMaterial, Mesh as ObjMesh};

/// Reads the Wavefront OBJ file at the given path and creates a corresponding
/// `TriangleMesh`. If there are multiple meshes in the file, they are merged
/// into a single mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn read_mesh_from_obj_file(file_path: impl AsRef<Path>) -> Result<TriangleMesh<f32>> {
    let file_path = file_path.as_ref();

    let (mut models, _) = tobj::load_obj(file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", file_path.display());
    }

    let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);

    for model in models {
        mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh));
    }

    Ok(mesh)
}

/// Reads the Wavefront OBJ file at the given path and any associated MTL
/// material files and returns the set of components representing the mesh and
/// material of each model in the file. The meshes are added to the mesh
/// repository, and any textures referenced in the MTL files are loaded as
/// rendering assets. Each [`ArchetypeComponentStorage`] in the returned list
/// contains the components describing a single model, and their order in the
/// list is the same as their order in the OBJ file.
///
/// # Errors
/// Returns an error if any of the involved OBJ, MTL or texture files can not be
/// found or loaded.
pub fn load_models_from_obj_file<P>(
    assets: &mut Assets,
    mesh_repository: &mut MeshRepository,
    obj_file_path: P,
) -> Result<Vec<SingleInstance<ArchetypeComponentStorage>>>
where
    P: AsRef<Path> + Debug,
{
    let obj_file_path = obj_file_path.as_ref();
    let obj_file_path_string = obj_file_path.to_string_lossy();

    let (models, materials) = tobj::load_obj(obj_file_path, &GPU_LOAD_OPTIONS)?;
    let materials = materials?;

    let mut model_components = Vec::with_capacity(models.len());
    let mut material_components = HashMap::new();

    for model in models {
        let material_id = model.mesh.material_id;

        let mesh_id = MeshID(hash64!(format!(
            "{} @ {}",
            &model.name, &obj_file_path_string
        )));

        if !mesh_repository.has_mesh(mesh_id) {
            let mesh = create_mesh_from_tobj_mesh(model.mesh);

            mesh_repository.add_mesh_unless_present(mesh_id, mesh);
        }

        let mesh_component = ComponentStorage::from_single_instance_view(&MeshComp { id: mesh_id });

        let components = if let Some(material_idx) = material_id {
            let material_components = match material_components.entry(material_idx) {
                Entry::Vacant(entry) => entry
                    .insert(create_material_components_from_tobj_material(
                        assets,
                        &obj_file_path_string,
                        &materials[material_idx],
                    )?)
                    .clone(),
                Entry::Occupied(entry) => entry.get().clone(),
            };

            let mut components = material_components;
            components.push(mesh_component);

            SingleInstance::<ArchetypeComponentStorage>::try_from_vec_of_single_instances(
                components,
            )
            .unwrap()
        } else {
            SingleInstance::<ArchetypeComponentStorage>::try_from_array_of_single_instances([
                mesh_component,
            ])
            .unwrap()
        };

        model_components.push(components);
    }

    Ok(model_components)
}

/// Reads the Wavefront OBJ file at the given path and adds the contained mesh
/// to the mesh repository if it does not already exist. If there are multiple
/// meshes in the file, they are merged into a single mesh.
///
/// # Returns
/// The [`MeshComp`] representing the mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn load_mesh_from_obj_file<P>(
    mesh_repository: &mut MeshRepository,
    obj_file_path: P,
) -> Result<MeshComp>
where
    P: AsRef<Path> + Debug,
{
    let obj_file_path = obj_file_path.as_ref();
    let obj_file_path_string = obj_file_path.to_string_lossy();

    let (mut models, _) = tobj::load_obj(obj_file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", obj_file_path_string);
    }

    let mesh_id = MeshID(hash64!(obj_file_path_string));

    if !mesh_repository.has_mesh(mesh_id) {
        let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);

        for model in models {
            mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh));
        }

        mesh_repository.add_mesh_unless_present(mesh_id, mesh);
    }

    Ok(MeshComp { id: mesh_id })
}

/// Reads the Wavefront OBJ file at the given path and adds the contained mesh
/// to the mesh repository if it does not already exist, after generating
/// texture coordinates for the mesh using the given projection. If there are
/// multiple meshes in the file, they are merged into a single mesh.
///
/// # Returns
/// The [`MeshComp`] representing the mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn load_mesh_from_obj_file_with_projection<P>(
    mesh_repository: &mut MeshRepository,
    obj_file_path: P,
    projection: &impl TextureProjection<f32>,
) -> Result<MeshComp>
where
    P: AsRef<Path> + Debug,
{
    let obj_file_path = obj_file_path.as_ref();
    let obj_file_path_string = obj_file_path.to_string_lossy();

    let (mut models, _) = tobj::load_obj(obj_file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", obj_file_path_string);
    }

    let mesh_id = MeshID(hash64!(format!(
        "{} (projection = {})",
        obj_file_path_string,
        projection.identifier()
    )));

    if !mesh_repository.has_mesh(mesh_id) {
        let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);

        for model in models {
            mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh));
        }

        mesh.generate_texture_coords(projection);

        mesh_repository.add_mesh_unless_present(mesh_id, mesh);
    }

    Ok(MeshComp { id: mesh_id })
}

fn create_mesh_from_tobj_mesh(mesh: ObjMesh) -> TriangleMesh<f32> {
    fn aggregate_3<T>(values: &[f32], aggregator: impl Fn(f32, f32, f32) -> T) -> Vec<T> {
        values
            .iter()
            .step_by(3)
            .zip(values.iter().skip(1).step_by(3))
            .zip(values.iter().skip(2).step_by(3))
            .map(|((&x, &y), &z)| aggregator(x, y, z))
            .collect()
    }

    fn aggregate_2<T>(values: &[f32], aggregator: impl Fn(f32, f32) -> T) -> Vec<T> {
        values
            .iter()
            .step_by(2)
            .zip(values.iter().skip(1).step_by(2))
            .map(|(&x, &y)| aggregator(x, y))
            .collect()
    }

    let positions = aggregate_3(&mesh.positions, |x, y, z| VertexPosition(point![x, y, z]));

    let normal_vectors = aggregate_3(&mesh.normals, |nx, ny, nz| {
        VertexNormalVector(UnitVector3::new_normalize(vector![nx, ny, nz]))
    });

    let texture_coords = aggregate_2(&mesh.texcoords, |u, v| VertexTextureCoords(vector![u, v]));

    TriangleMesh::new(
        positions,
        normal_vectors,
        texture_coords,
        Vec::new(),
        mesh.indices,
    )
}

fn create_material_components_from_tobj_material(
    assets: &mut Assets,
    obj_file_path: impl AsRef<str>,
    material: &ObjMaterial,
) -> Result<Vec<SingleInstance<ComponentStorage>>> {
    let obj_file_path = obj_file_path.as_ref();

    match material.dissolve {
        Some(alpha) if alpha != 1.0 => {
            log::warn!(
                "Warning: Unsupported content in MTL material referenced in {}: \
                 material `{}` uses a value for alpha not equal to 1 ({}) - alpha will be ignored",
                obj_file_path,
                &material.name,
                alpha,
            );
        }
        _ => {}
    }
    if let Some(alpha_texture_path) = &material.dissolve_texture {
        log::warn!(
            "Warning: Unsupported content in MTL material referenced in {}: \
             material `{}` uses a texture for alpha ({}) - alpha will be ignored",
            obj_file_path,
            &material.name,
            alpha_texture_path,
        );
    }
    if let Some(ambient_color) = material.ambient {
        log::warn!(
            "Warning: Unsupported content in MTL material referenced in {}: \
             material `{}` uses an ambient color ({:?}) - ambient color will be ignored",
            obj_file_path,
            &material.name,
            ambient_color,
        );
    }
    if let Some(ambient_texture_path) = &material.ambient_texture {
        log::warn!(
            "Warning: Unsupported content in MTL material referenced in {}: \
             material `{}` uses a texture for ambient color ({}) - ambient color will be ignored",
            obj_file_path,
            &material.name,
            ambient_texture_path,
        );
    }
    if let Some(shininess_texture_path) = &material.shininess_texture {
        log::warn!(
            "Warning: Unsupported content in MTL material referenced in {}: \
             material `{}` uses a texture for shininess ({}) - falling back to fixed value of {} instead",
            obj_file_path,
            &material.name,
            shininess_texture_path,
            material.shininess.unwrap_or(0.0)
        );
    }

    let mut components = Vec::with_capacity(4);

    if let Some(albedo_texture_path) = &material.diffuse_texture {
        let albedo_texture_id = assets.load_texture_from_path(
            albedo_texture_path,
            albedo_texture_path,
            TextureConfig {
                color_space: ColorSpace::Srgb,
                ..Default::default()
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::Repeating,
                ..Default::default()
            }),
        )?;

        components.push(ComponentStorage::from_single_instance_view(
            &TexturedColorComp(albedo_texture_id),
        ));
    } else {
        components.push(ComponentStorage::from_single_instance_view(
            &UniformColorComp(RGBColor::from_row_slice(
                &material.diffuse.unwrap_or([0.0; 3]),
            )),
        ));
    }

    if let Some(specular_reflectance_path) = &material.specular_texture {
        let specular_reflectance_id = assets.load_texture_from_path(
            specular_reflectance_path,
            specular_reflectance_path,
            TextureConfig {
                color_space: ColorSpace::Srgb,
                ..Default::default()
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::Repeating,
                ..Default::default()
            }),
        )?;

        components.push(ComponentStorage::from_single_instance_view(
            &TexturedSpecularReflectanceComp::unscaled(specular_reflectance_id),
        ));
    }

    if let Some(normal_texture_path) = &material.normal_texture {
        let normal_texture_id = assets.load_texture_from_path(
            normal_texture_path,
            normal_texture_path,
            TextureConfig {
                color_space: ColorSpace::Linear,
                ..Default::default()
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::Repeating,
                ..Default::default()
            }),
        )?;

        components.push(ComponentStorage::from_single_instance_view(&NormalMapComp(
            normal_texture_id,
        )));
    }

    Ok(components)
}
