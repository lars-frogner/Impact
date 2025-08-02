/// Convenience macro for implementing the
/// [`InstanceFeature`](crate::InstanceFeature) trait. The feature type ID is
/// created by hashing the name of the implementing type. Buffers for this type
/// will not get associated GPU buffers.
#[macro_export]
macro_rules! impl_InstanceFeature {
    ($ty:ty) => {
        impl $crate::InstanceFeature for $ty {
            const FEATURE_TYPE_ID: $crate::InstanceFeatureTypeID =
                $crate::InstanceFeatureTypeID::from_hash(
                    impact_math::ConstStringHash64::new(stringify!($ty)).into_hash(),
                );

            const BUFFER_LAYOUT: Option<::impact_gpu::wgpu::VertexBufferLayout<'static>> = None;
        }
    };
}

/// Convenience macro for implementing the
/// [`InstanceFeature`](crate::InstanceFeature) trait. The feature type ID is
/// created by hashing the name of the implementing type. Buffers for this type
/// will get associated GPU buffers.
#[macro_export]
macro_rules! impl_InstanceFeatureForGPU {
    ($ty:ty, $vertex_attr_array:expr) => {
        impl $crate::InstanceFeature for $ty {
            const FEATURE_TYPE_ID: $crate::InstanceFeatureTypeID =
                $crate::InstanceFeatureTypeID::from_hash(
                    impact_math::ConstStringHash64::new(stringify!($ty)).into_hash(),
                );

            const BUFFER_LAYOUT: Option<::impact_gpu::wgpu::VertexBufferLayout<'static>> =
                Some(::impact_gpu::wgpu::VertexBufferLayout {
                    array_stride: ::std::mem::size_of::<Self>()
                        as ::impact_gpu::wgpu::BufferAddress,
                    step_mode: ::impact_gpu::wgpu::VertexStepMode::Instance,
                    attributes: &$vertex_attr_array,
                });
        }
    };
}
