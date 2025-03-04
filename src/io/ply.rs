//! Input/output of mesh data in Polygon File Format.

use crate::mesh::{
    MeshID, MeshRepository, TriangleMesh, VertexNormalVector, VertexPosition, VertexTextureCoords,
    components::MeshComp, texture_projection::TextureProjection,
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_utils::hash64;
use nalgebra::{UnitVector3, point, vector};
use ply_rs::{
    parser::Parser,
    ply::{Property, PropertyAccess},
};
use std::{fmt::Debug, fs::File, io::BufReader, path::Path};

#[derive(Clone, Debug)]
struct PlyVertex {
    property_values: Vec<f32>,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct PlyTriangleVertexIndices([u32; 3]);

/// Reads the PLY (Polygon File Format, also called Stanford Triangle Format)
/// file at the given path and adds the contained mesh to the mesh repository if
/// it does not already exist.
///
/// # Returns
/// The [`MeshComp`] representing the mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn load_mesh_from_ply_file<P>(
    mesh_repository: &mut MeshRepository<f32>,
    ply_file_path: P,
) -> Result<MeshComp>
where
    P: AsRef<Path> + Debug,
{
    let ply_file_path = ply_file_path.as_ref();

    let mesh_id = MeshID(hash64!(ply_file_path.to_string_lossy()));

    if !mesh_repository.has_mesh(mesh_id) {
        let mesh = read_mesh_from_ply_file(ply_file_path)?;

        mesh_repository.add_mesh_unless_present(mesh_id, mesh);
    }

    Ok(MeshComp { id: mesh_id })
}

/// Reads the PLY (Polygon File Format, also called Stanford Triangle Format)
/// file at the given path and adds the contained mesh to the mesh repository if
/// it does not already exist, after generating texture coordinates for the mesh
/// using the given projection.
///
/// # Returns
/// The [`MeshComp`] representing the mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn load_mesh_from_ply_file_with_projection<P>(
    mesh_repository: &mut MeshRepository<f32>,
    ply_file_path: P,
    projection: &impl TextureProjection<f32>,
) -> Result<MeshComp>
where
    P: AsRef<Path> + Debug,
{
    let ply_file_path = ply_file_path.as_ref();

    let mesh_id = MeshID(hash64!(format!(
        "{} (projection = {})",
        ply_file_path.display(),
        projection.identifier()
    )));

    if !mesh_repository.has_mesh(mesh_id) {
        let mut mesh = read_mesh_from_ply_file(ply_file_path)?;

        mesh.generate_texture_coords(projection);

        mesh_repository.add_mesh_unless_present(mesh_id, mesh);
    }

    Ok(MeshComp { id: mesh_id })
}

pub fn read_mesh_from_ply_file<P>(ply_file_path: P) -> Result<TriangleMesh<f32>>
where
    P: AsRef<Path> + Debug,
{
    let vertex_parser = Parser::<PlyVertex>::new();
    let triangle_vertex_indices_parser = Parser::<PlyTriangleVertexIndices>::new();

    let mut file_reader = BufReader::new(File::open(ply_file_path.as_ref())?);

    let header = vertex_parser.read_header(&mut file_reader)?;

    let mut vertex_property_names = Vec::new();
    let mut vertex_list = Vec::new();
    let mut triangle_vertex_indices_list = Vec::new();

    for element in header.elements.values() {
        match element.name.as_str() {
            "vertex" => {
                vertex_property_names.extend(element.properties.keys());
                vertex_list =
                    vertex_parser.read_payload_for_element(&mut file_reader, element, &header)?;
            }
            "face" => {
                triangle_vertex_indices_list = triangle_vertex_indices_parser
                    .read_payload_for_element(&mut file_reader, element, &header)?;
            }
            element_name => bail!(
                "Unexpected element `{}` in header of {}",
                element_name,
                ply_file_path.as_ref().display()
            ),
        }
    }

    Ok(convert_ply_vertices_and_faces_to_mesh(
        vertex_property_names,
        vertex_list,
        triangle_vertex_indices_list,
    ))
}

fn convert_ply_vertices_and_faces_to_mesh(
    vertex_property_names: Vec<&String>,
    vertex_list: Vec<PlyVertex>,
    triangle_vertex_indices_list: Vec<PlyTriangleVertexIndices>,
) -> TriangleMesh<f32> {
    let mut prop_idx = 0;

    let mut vertex_positions = Vec::new();
    let mut vertex_normal_vectors = Vec::new();
    let mut vertex_texture_coords = Vec::new();

    if prop_idx + 3 <= vertex_property_names.len() {
        if let ("x", "y", "z") = (
            vertex_property_names[prop_idx].as_str(),
            vertex_property_names[prop_idx + 1].as_str(),
            vertex_property_names[prop_idx + 2].as_str(),
        ) {
            vertex_positions = vertex_list
                .iter()
                .map(|PlyVertex { property_values }| {
                    VertexPosition(point![
                        property_values[prop_idx],
                        property_values[prop_idx + 1],
                        property_values[prop_idx + 2]
                    ])
                })
                .collect();

            prop_idx += 3;
        }
    }

    if prop_idx + 3 <= vertex_property_names.len() {
        if let ("red", "green", "blue") = (
            vertex_property_names[prop_idx].as_str(),
            vertex_property_names[prop_idx + 1].as_str(),
            vertex_property_names[prop_idx + 2].as_str(),
        ) {
            // Ignore vertex colors
            prop_idx += 3;
        }
    }

    if prop_idx + 3 <= vertex_property_names.len() {
        if let ("nx", "ny", "nz") = (
            vertex_property_names[prop_idx].as_str(),
            vertex_property_names[prop_idx + 1].as_str(),
            vertex_property_names[prop_idx + 2].as_str(),
        ) {
            vertex_normal_vectors = vertex_list
                .iter()
                .map(|PlyVertex { property_values }| {
                    VertexNormalVector(UnitVector3::new_normalize(vector![
                        property_values[prop_idx],
                        property_values[prop_idx + 1],
                        property_values[prop_idx + 2]
                    ]))
                })
                .collect();

            prop_idx += 3;
        }
    }

    if prop_idx + 2 <= vertex_property_names.len() {
        if let ("texture_u" | "u", "texture_v" | "v") = (
            vertex_property_names[prop_idx].as_str(),
            vertex_property_names[prop_idx + 1].as_str(),
        ) {
            vertex_texture_coords = vertex_list
                .iter()
                .map(|PlyVertex { property_values }| {
                    VertexTextureCoords(vector![
                        property_values[prop_idx],
                        property_values[prop_idx + 1]
                    ])
                })
                .collect();

            prop_idx += 2;
        }
    }

    assert_eq!(prop_idx, vertex_property_names.len());

    let triangle_vertex_indices: Vec<u32> = bytemuck::cast_vec(triangle_vertex_indices_list);

    TriangleMesh::new(
        vertex_positions,
        vertex_normal_vectors,
        vertex_texture_coords,
        Vec::new(),
        triangle_vertex_indices,
    )
}

impl PropertyAccess for PlyVertex {
    fn new() -> Self {
        Self {
            property_values: Vec::with_capacity(8), /* 8 = position + normal vector + texture
                                                     * coordinates */
        }
    }

    fn set_property(&mut self, property_name: String, property: Property) {
        match (property_name.as_str(), property) {
            (
                "x" | "y" | "z" | "red" | "green" | "blue" | "nx" | "ny" | "nz" | "texture_u" | "u"
                | "texture_v" | "v",
                Property::Float(value),
            ) => {
                self.property_values.push(value);
            }
            (
                "x" | "y" | "z" | "red" | "green" | "blue" | "nx" | "ny" | "nz" | "texture_u" | "u"
                | "texture_v" | "v",
                Property::Double(value),
            ) => {
                self.property_values.push(value as f32);
            }
            ("red" | "green" | "blue", Property::UChar(color)) => {
                self.property_values.push(f32::from(color) / 255.0);
            }
            (
                "x" | "y" | "z" | "red" | "green" | "blue" | "nx" | "ny" | "nz" | "texture_u" | "u"
                | "texture_v" | "v",
                property,
            ) => panic!(
                "Unsupported format for vertex property `{}` in PLY file: {:?}",
                &property_name, property
            ),
            _ => panic!(
                "Unsupported vertex property in PLY file: {}",
                &property_name
            ),
        }
    }
}

impl PropertyAccess for PlyTriangleVertexIndices {
    fn new() -> Self {
        Self([0; 3])
    }

    fn set_property(&mut self, property_name: String, property: Property) {
        let verify_length = |length: usize| {
            assert_eq!(
                length, 3,
                "Tried to set triangle vertex indices with list of {} indices",
                length
            );
        };

        match (property_name.as_str(), property) {
            ("vertex_index" | "vertex_indices", Property::ListUChar(indices)) => {
                verify_length(indices.len());
                self.0[0] = u32::from(indices[0]);
                self.0[1] = u32::from(indices[1]);
                self.0[2] = u32::from(indices[2]);
            }
            ("vertex_index" | "vertex_indices", Property::ListChar(indices)) => {
                verify_length(indices.len());
                self.0[0] = u32::try_from(indices[0]).unwrap();
                self.0[1] = u32::try_from(indices[1]).unwrap();
                self.0[2] = u32::try_from(indices[2]).unwrap();
            }
            ("vertex_index" | "vertex_indices", Property::ListUShort(indices)) => {
                verify_length(indices.len());
                self.0[0] = u32::from(indices[0]);
                self.0[1] = u32::from(indices[1]);
                self.0[2] = u32::from(indices[2]);
            }
            ("vertex_index" | "vertex_indices", Property::ListShort(indices)) => {
                verify_length(indices.len());
                self.0[0] = u32::try_from(indices[0]).unwrap();
                self.0[1] = u32::try_from(indices[1]).unwrap();
                self.0[2] = u32::try_from(indices[2]).unwrap();
            }
            ("vertex_index" | "vertex_indices", Property::ListUInt(indices)) => {
                verify_length(indices.len());
                self.0[0] = indices[0];
                self.0[1] = indices[1];
                self.0[2] = indices[2];
            }
            ("vertex_index" | "vertex_indices", Property::ListInt(indices)) => {
                verify_length(indices.len());
                self.0[0] = u32::try_from(indices[0]).unwrap();
                self.0[1] = u32::try_from(indices[1]).unwrap();
                self.0[2] = u32::try_from(indices[2]).unwrap();
            }
            _ => panic!(
                "Tried to set unexpected property for PlyTriangleVertexIndices: {}",
                &property_name
            ),
        }
    }
}
