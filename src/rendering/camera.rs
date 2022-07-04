//! Camera representation for rendering.

use crate::{geometry::Camera3, num::Float};
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;

/// Camera uniform for use in shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct CameraUniform {
    view_projection_matrix: [[f32; 4]; 4],
}

impl CameraUniform {
    /// Creates a new camera uniform for the given camera.
    pub fn from_camera<C, F>(camera: &C) -> Self
    where
        C: Camera3<F>,
        F: Float + simba::scalar::SubsetOf<f32>,
    {
        Self::from_matrix(camera.create_view_projection_transform().matrix())
    }

    /// Creates a new camera uniform with the given view
    /// projection matrix.
    pub fn from_matrix<F>(view_projection_matrix: &Matrix4<F>) -> Self
    where
        F: Float + simba::scalar::SubsetOf<f32>,
    {
        Self::from_nested_slice(*view_projection_matrix.cast().as_ref())
    }

    pub fn from_nested_slice(view_projection_matrix: [[f32; 4]; 4]) -> Self {
        Self {
            view_projection_matrix,
        }
    }
}
