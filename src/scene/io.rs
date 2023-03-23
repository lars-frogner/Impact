//! Input/output of scene data.

use crate::{
    geometry::{
        TriangleMesh, VertexColor, VertexNormalVector, VertexPosition, VertexTextureCoords,
    },
    rendering::{fre, RenderingSystem},
    scene::{MeshComp, MeshRepository, VertexColorComp},
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentStorage, SingleInstance},
};
use nalgebra::{point, vector, UnitVector3};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::Path,
    sync::RwLock,
};
use tobj::{Material as ObjMaterial, Mesh as ObjMesh, GPU_LOAD_OPTIONS};

use super::material::{
    BlinnPhongShininessComp, BlinnPhongSpecularColorComp, BlinnPhongSpecularTextureComp,
    LambertianDiffuseColorComp, LambertianDiffuseTextureComp,
};

/// Reads the Wavefront OBJ file at the given path and any associated MTL
/// material files and returns the set of components representing the mesh and
/// material of each model in the file. The meshes are added to the mesh
/// repository, and any image textures referenced in the MTL files are loaded as
/// rendering assets. Each [`ArchetypeComponentStorage`] in the returned list
/// contains the components describing a single model, and their order in the
/// list is the same as their order in the OBJ file.
///
/// # Errors
/// Returns an error if any of the involved OBJ, MTL or texture files can not be
/// found or loaded.
pub fn load_models_from_obj_file<P>(
    renderer: &RwLock<RenderingSystem>,
    mesh_repository: &RwLock<MeshRepository<fre>>,
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

        let mesh = create_mesh_from_tobj_mesh(model.mesh);
        let mesh_has_vertex_colors = mesh.has_colors();

        let mesh_name = format!("{} @ {}", &model.name, &obj_file_path_string);
        let mesh_id = mesh_repository
            .write()
            .unwrap()
            .add_named_mesh_unless_present(mesh_name, mesh);
        let mesh_component = ComponentStorage::from_single_instance_view(&MeshComp { id: mesh_id });

        let components = if let Some(material_idx) = material_id {
            let material_components = match material_components.entry(material_idx) {
                Entry::Vacant(entry) => entry
                    .insert(create_material_components_from_tobj_material(
                        renderer,
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
        mesh.indices,
    )
}

fn create_material_components_from_tobj_material(
    renderer: &RwLock<RenderingSystem>,
    obj_file_path: impl AsRef<str>,
    material: &ObjMaterial,
) -> Result<Vec<SingleInstance<ComponentStorage>>> {
    let obj_file_path = obj_file_path.as_ref();

    if !material.dissolve_texture.is_empty() {
        log::warn!(
            "Warning: Unsupported MTL material referenced in {}: \
             material `{}` uses a texture for alpha ({}) - falling back to fixed value of {} instead",
            obj_file_path,
            &material.name,
            &material.dissolve_texture,
            material.dissolve
        );
    }
    if !material.shininess_texture.is_empty() {
        log::warn!(
            "Warning: Unsupported MTL material referenced in {}: \
             material `{}` uses a texture for shininess ({}) - falling back to fixed value of {} instead",
             obj_file_path,
             &material.name,
            &material.normal_texture,
            material.shininess
        );
    }
    if !material.normal_texture.is_empty() {
        log::warn!(
            "Warning: Unsupported MTL material referenced in {}: \
             material `{}` uses a texture for normals ({}) - falling back to mesh normals instead",
            obj_file_path,
            &material.name,
            &material.normal_texture,
        );
    }
    if !material.ambient_texture.is_empty() {
        log::warn!(
            "Warning: Unsupported MTL material referenced in {}: \
             material `{}` uses a texture for ambient color ({}) - falling back to fixed value of {:?} instead",
             obj_file_path,
             &material.name,
            &material.normal_texture,
            &material.ambient
        );
    }

    let mut components = Vec::with_capacity(3);

    if material.diffuse_texture.is_empty() {
        components.push(ComponentStorage::from_single_instance_view(
            &LambertianDiffuseColorComp(material.diffuse.into()),
        ));
    } else {
        let renderer = renderer.read().unwrap();
        let mut assets = renderer.assets().write().unwrap();

        let diffuse_texture_id = assets
            .load_image_texture_from_path(renderer.core_system(), &material.diffuse_texture)?;

        components.push(ComponentStorage::from_single_instance_view(
            &LambertianDiffuseTextureComp(diffuse_texture_id),
        ));
    }

    if material.specular_texture.is_empty() {
        components.push(ComponentStorage::from_single_instance_view(
            &BlinnPhongSpecularColorComp(material.specular.into()),
        ));
    } else {
        let renderer = renderer.read().unwrap();
        let mut assets = renderer.assets().write().unwrap();

        let specular_texture_id = assets
            .load_image_texture_from_path(renderer.core_system(), &material.specular_texture)?;

        components.push(ComponentStorage::from_single_instance_view(
            &BlinnPhongSpecularTextureComp(specular_texture_id),
        ));
    }

    components.push(ComponentStorage::from_single_instance_view(
        &BlinnPhongShininessComp(material.shininess),
    ));

    Ok(components)
}
