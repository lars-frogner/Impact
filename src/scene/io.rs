//! Input/output of scene data.

use crate::{
    geometry::{
        TriangleMesh, VertexColor, VertexNormalVector, VertexPosition, VertexTextureCoords,
    },
    rendering::{fre, Assets, CoreRenderingSystem, RenderingSystem},
    scene::{
        BlinnPhongComp, DiffuseTexturedBlinnPhongComp, MeshComp, MeshRepository,
        TexturedBlinnPhongComp, VertexColorComp,
    },
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
            let material_component = match material_components.entry(material_idx) {
                Entry::Vacant(entry) => entry
                    .insert(create_material_component_from_tobj_material(
                        renderer,
                        &obj_file_path_string,
                        &materials[material_idx],
                    )?)
                    .clone(),
                Entry::Occupied(entry) => entry.get().clone(),
            };

            SingleInstance::<ArchetypeComponentStorage>::try_from_single_instances([
                mesh_component,
                material_component,
            ])
            .unwrap()
        } else if mesh_has_vertex_colors {
            let material_component = ComponentStorage::from_single_instance_view(&VertexColorComp);

            SingleInstance::<ArchetypeComponentStorage>::try_from_single_instances([
                mesh_component,
                material_component,
            ])
            .unwrap()
        } else {
            SingleInstance::<ArchetypeComponentStorage>::try_from_single_instances([mesh_component])
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

    let colors = aggregate_3(&mesh.vertex_color, |r, g, b| {
        VertexColor(vector![r, g, b, 1.0])
    });

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

fn create_material_component_from_tobj_material(
    renderer: &RwLock<RenderingSystem>,
    obj_file_path: impl AsRef<str>,
    material: &ObjMaterial,
) -> Result<SingleInstance<ComponentStorage>> {
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
    if material.specular_texture.is_empty() {
        if material.diffuse_texture.is_empty() {
            Ok(create_blinn_phong_material_component_from_tobj_material(
                material,
            ))
        } else {
            let renderer = renderer.read().unwrap();
            let mut assets = renderer.assets().write().unwrap();
            create_diffuse_textured_blinn_phong_material_component_from_tobj_material(
                renderer.core_system(),
                &mut assets,
                material,
            )
        }
    } else if material.diffuse_texture.is_empty() {
        log::warn!(
            "Warning: Unsupported MTL material referenced in {}: \
             material `{}` uses a texture for specular color but not for diffuse -  falling back to fixed specular color of {:?} instead",
             obj_file_path,
             &material.name,
            &material.specular
        );
        Ok(create_blinn_phong_material_component_from_tobj_material(
            material,
        ))
    } else {
        let renderer = renderer.read().unwrap();
        let mut assets = renderer.assets().write().unwrap();
        create_textured_blinn_phong_material_component_from_tobj_material(
            renderer.core_system(),
            &mut assets,
            material,
        )
    }
}

fn create_blinn_phong_material_component_from_tobj_material(
    material: &ObjMaterial,
) -> SingleInstance<ComponentStorage> {
    ComponentStorage::from_single_instance_view(&BlinnPhongComp {
        ambient: material.ambient.into(),
        diffuse: material.diffuse.into(),
        specular: material.specular.into(),
        shininess: material.shininess,
        alpha: material.dissolve,
    })
}

fn create_diffuse_textured_blinn_phong_material_component_from_tobj_material(
    core_system: &CoreRenderingSystem,
    assets: &mut Assets,
    material: &ObjMaterial,
) -> Result<SingleInstance<ComponentStorage>> {
    let diffuse_texture_id =
        assets.load_image_texture_from_path(core_system, &material.diffuse_texture)?;

    Ok(ComponentStorage::from_single_instance_view(
        &DiffuseTexturedBlinnPhongComp {
            ambient: material.ambient.into(),
            diffuse: diffuse_texture_id,
            specular: material.specular.into(),
            shininess: material.shininess,
            alpha: material.dissolve,
        },
    ))
}

fn create_textured_blinn_phong_material_component_from_tobj_material(
    core_system: &CoreRenderingSystem,
    assets: &mut Assets,
    material: &ObjMaterial,
) -> Result<SingleInstance<ComponentStorage>> {
    let diffuse_texture_id =
        assets.load_image_texture_from_path(core_system, &material.diffuse_texture)?;
    let specular_texture_id =
        assets.load_image_texture_from_path(core_system, &material.specular_texture)?;

    Ok(ComponentStorage::from_single_instance_view(
        &TexturedBlinnPhongComp {
            ambient: material.ambient.into(),
            diffuse: diffuse_texture_id,
            specular: specular_texture_id,
            shininess: material.shininess,
            alpha: material.dissolve,
        },
    ))
}