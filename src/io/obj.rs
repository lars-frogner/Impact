//! Input/output of mesh data in Wavefront OBJ format.

use crate::{
    assets::Assets,
    geometry::{
        TextureProjection, TriangleMesh, VertexColor, VertexNormalVector, VertexPosition,
        VertexTextureCoords,
    },
    gpu::{
        rendering::{fre, ColorSpace, TextureAddressingConfig, TextureConfig},
        GraphicsDevice,
    },
    material::{
        AlbedoComp, AlbedoTextureComp, NormalMapComp, RGBColor, RoughnessComp,
        SpecularReflectanceComp, SpecularReflectanceTextureComp, VertexColorComp,
    },
    scene::{MeshComp, MeshID, MeshRepository},
};
use anyhow::{bail, Result};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentStorage, SingleInstance},
};
use impact_utils::hash64;
use nalgebra::{point, vector, UnitVector3};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::Path,
};
use tobj::{Material as ObjMaterial, Mesh as ObjMesh, GPU_LOAD_OPTIONS};

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
    graphics_device: &GraphicsDevice,
    assets: &mut Assets,
    mesh_repository: &mut MeshRepository<fre>,
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

        let mesh_has_vertex_colors = !model.mesh.vertex_color.is_empty();

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
                        graphics_device,
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
        } else if mesh_has_vertex_colors {
            let material_component = ComponentStorage::from_single_instance_view(&VertexColorComp);

            SingleInstance::<ArchetypeComponentStorage>::try_from_array_of_single_instances([
                mesh_component,
                material_component,
            ])
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
    mesh_repository: &mut MeshRepository<fre>,
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
    mesh_repository: &mut MeshRepository<fre>,
    obj_file_path: P,
    projection: &impl TextureProjection<fre>,
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

pub fn read_meshes_from_obj_file<P>(obj_file_path: P) -> Result<Vec<TriangleMesh<fre>>>
where
    P: AsRef<Path> + Debug,
{
    let (models, _) = tobj::load_obj(obj_file_path, &GPU_LOAD_OPTIONS)?;
    Ok(models
        .into_iter()
        .map(|model| create_mesh_from_tobj_mesh(model.mesh))
        .collect())
}

fn create_mesh_from_tobj_mesh(mesh: ObjMesh) -> TriangleMesh<fre> {
    fn aggregate_3<T>(values: &[fre], aggregator: impl Fn(fre, fre, fre) -> T) -> Vec<T> {
        values
            .iter()
            .step_by(3)
            .zip(values.iter().skip(1).step_by(3))
            .zip(values.iter().skip(2).step_by(3))
            .map(|((&x, &y), &z)| aggregator(x, y, z))
            .collect()
    }

    fn aggregate_2<T>(values: &[fre], aggregator: impl Fn(fre, fre) -> T) -> Vec<T> {
        values
            .iter()
            .step_by(2)
            .zip(values.iter().skip(1).step_by(2))
            .map(|(&x, &y)| aggregator(x, y))
            .collect()
    }

    let positions = aggregate_3(&mesh.positions, |x, y, z| VertexPosition(point![x, y, z]));

    let colors = aggregate_3(&mesh.vertex_color, |r, g, b| VertexColor(vector![r, g, b]));

    let normal_vectors = aggregate_3(&mesh.normals, |nx, ny, nz| {
        VertexNormalVector(UnitVector3::new_normalize(vector![nx, ny, nz]))
    });

    let texture_coords = aggregate_2(&mesh.texcoords, |u, v| VertexTextureCoords(vector![u, v]));

    TriangleMesh::new(
        positions,
        colors,
        normal_vectors,
        texture_coords,
        Vec::new(),
        mesh.indices,
    )
}

fn create_material_components_from_tobj_material(
    graphics_device: &GraphicsDevice,
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
            graphics_device,
            albedo_texture_path,
            TextureConfig {
                color_space: ColorSpace::Srgb,
                addressing: TextureAddressingConfig::REPEATING,
                ..Default::default()
            },
        )?;

        components.push(ComponentStorage::from_single_instance_view(
            &AlbedoTextureComp(albedo_texture_id),
        ));
    } else {
        components.push(ComponentStorage::from_single_instance_view(&AlbedoComp(
            RGBColor::from_row_slice(&material.diffuse.unwrap_or([0.0; 3])),
        )));
    }

    if let Some(specular_reflectance_path) = &material.specular_texture {
        let specular_reflectance_id = assets.load_texture_from_path(
            graphics_device,
            specular_reflectance_path,
            TextureConfig {
                color_space: ColorSpace::Srgb,
                addressing: TextureAddressingConfig::REPEATING,
                ..Default::default()
            },
        )?;

        components.push(ComponentStorage::from_single_instance_view(
            &SpecularReflectanceTextureComp(specular_reflectance_id),
        ));
    } else {
        components.push(ComponentStorage::from_single_instance_view(
            &SpecularReflectanceComp(RGBColor::from_row_slice(
                &material.specular.unwrap_or([0.0; 3]),
            )),
        ));
    }

    if let Some(normal_texture_path) = &material.normal_texture {
        let normal_texture_id = assets.load_texture_from_path(
            graphics_device,
            normal_texture_path,
            TextureConfig {
                color_space: ColorSpace::Linear,
                addressing: TextureAddressingConfig::REPEATING,
                ..Default::default()
            },
        )?;

        components.push(ComponentStorage::from_single_instance_view(&NormalMapComp(
            normal_texture_id,
        )));
    }

    components.push(ComponentStorage::from_single_instance_view(
        &RoughnessComp::from_blinn_phong_shininess(material.shininess.unwrap_or(0.0)),
    ));

    Ok(components)
}
