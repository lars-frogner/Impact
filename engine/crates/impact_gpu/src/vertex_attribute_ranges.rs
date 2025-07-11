//! Vertex attribute location range constants.

/// Size of the instance vertex attribute range.
pub const INSTANCE_RANGE_SIZE: u32 = 4;

/// Size of the mesh vertex attribute range.
pub const MESH_RANGE_SIZE: u32 = 5;

/// Size of the material vertex attribute range.
pub const MATERIAL_RANGE_SIZE: u32 = 7;

/// Starting location for instance vertex attributes.
pub const INSTANCE_START: u32 = 0;

/// Starting location for mesh vertex attributes.
pub const MESH_START: u32 = INSTANCE_START + INSTANCE_RANGE_SIZE;

/// Starting location for material vertex attributes.
pub const MATERIAL_START: u32 = MESH_START + MESH_RANGE_SIZE;

/// Total number of vertex attribute locations used.
pub const TOTAL_LOCATIONS: u32 = MATERIAL_START + MATERIAL_RANGE_SIZE;
