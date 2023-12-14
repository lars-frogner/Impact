//! Management of model instances.

use crate::{
    geometry::{
        DynamicInstanceFeatureBuffer, InstanceFeature, InstanceFeatureID, InstanceFeatureStorage,
        InstanceFeatureTypeID, InstanceModelViewTransform,
    },
    scene::{MaterialLibrary, ModelID},
};
use std::{collections::HashMap, fmt::Debug};

/// Container for features associated with instances of specific models.
///
/// Holds a set of [`InstanceFeatureStorage`]s, one storage for each feature
/// type. These storages are presistent and can be accessed to add, remove or
/// modify feature values for individual instances.
///
/// Additionally, a set of [`DynamicInstanceFeatureBuffer`]s is kept for each
/// model that has instances, one buffer for each feature type associated with
/// the model, with the first one always being a buffer for model-to-camera
/// transforms. These buffers are filled with transforms as well as feature
/// values from the `InstanceFeatureStorage`s for all instances that are to be
/// rendered. Their contents can then be copied directly to the corresponding
/// render buffers, before they are cleared in preparation for the next frame.
#[derive(Debug, Default)]
pub struct InstanceFeatureManager {
    feature_storages: HashMap<InstanceFeatureTypeID, InstanceFeatureStorage>,
    instance_feature_buffers: HashMap<ModelID, (usize, Vec<DynamicInstanceFeatureBuffer>)>,
}

impl InstanceFeatureManager {
    /// Creates a new empty instance feature manager.
    pub fn new() -> Self {
        Self {
            feature_storages: HashMap::new(),
            instance_feature_buffers: HashMap::new(),
        }
    }

    /// Whether the manager has instance feature buffers for the model with the
    /// given ID.
    pub fn has_model_id(&self, model_id: ModelID) -> bool {
        self.instance_feature_buffers.contains_key(&model_id)
    }

    /// Returns a reference to the set of instance feature buffers for the model
    /// with the given ID, or [`None`] if the model is not present.
    pub fn get_buffers(&self, model_id: ModelID) -> Option<&Vec<DynamicInstanceFeatureBuffer>> {
        self.instance_feature_buffers
            .get(&model_id)
            .map(|(_, buffers)| buffers)
    }

    /// Returns a mutable iterator over each buffer of model instance
    /// transforms.
    pub fn transform_buffers_mut(
        &mut self,
    ) -> impl Iterator<Item = &'_ mut DynamicInstanceFeatureBuffer> {
        self.instance_feature_buffers.values_mut().map(|buffers| {
            buffers
                .1
                .get_mut(0)
                .expect("Missing transform buffer for model")
        })
    }

    /// Returns a reference to the storage of instance features of type `Fe`, or
    /// [`None`] if no storage exists for that type.
    pub fn get_storage<Fe: InstanceFeature>(&self) -> Option<&InstanceFeatureStorage> {
        self.get_storage_for_feature_type_id(Fe::FEATURE_TYPE_ID)
    }

    /// Returns a mutable reference to the storage of instance features
    /// of type `Fe`, or [`None`] if no storage exists for that type.
    pub fn get_storage_mut<Fe: InstanceFeature>(&mut self) -> Option<&mut InstanceFeatureStorage> {
        self.get_storage_mut_for_feature_type_id(Fe::FEATURE_TYPE_ID)
    }

    /// Returns a reference to the storage of instance features of the type with
    /// the given ID, or [`None`] if no storage exists for that type.
    pub fn get_storage_for_feature_type_id(
        &self,
        feature_type_id: InstanceFeatureTypeID,
    ) -> Option<&InstanceFeatureStorage> {
        self.feature_storages.get(&feature_type_id)
    }

    /// Returns a mutable reference to the storage of instance features of the
    /// type with the given ID, or [`None`] if no storage exists for that type.
    pub fn get_storage_mut_for_feature_type_id(
        &mut self,
        feature_type_id: InstanceFeatureTypeID,
    ) -> Option<&mut InstanceFeatureStorage> {
        self.feature_storages.get_mut(&feature_type_id)
    }

    /// Returns an iterator over the model IDs and their associated sets of
    /// instance feature buffers.
    pub fn model_ids_and_buffers(
        &self,
    ) -> impl Iterator<Item = (ModelID, &'_ Vec<DynamicInstanceFeatureBuffer>)> {
        self.instance_feature_buffers
            .iter()
            .map(|(model_id, (_, buffers))| (*model_id, buffers))
    }

    /// Returns an iterator over the model IDs and their associated sets of
    /// instance feature buffers, with the buffers being mutable.
    pub fn model_ids_and_mutable_buffers(
        &mut self,
    ) -> impl Iterator<Item = (ModelID, &'_ mut Vec<DynamicInstanceFeatureBuffer>)> {
        self.instance_feature_buffers
            .iter_mut()
            .map(|(model_id, (_, buffers))| (*model_id, buffers))
    }

    /// Sets up a storage for features of type `Fe`, which is required for
    /// supporting instances with features of that type.
    ///
    /// If a storage for the feature type is already set up, nothing happens.
    pub fn register_feature_type<Fe: InstanceFeature>(&mut self) {
        self.feature_storages
            .entry(Fe::FEATURE_TYPE_ID)
            .or_insert_with(|| InstanceFeatureStorage::new::<Fe>());
    }

    /// Registers the existance of a new instance of the model with the given
    /// ID. This involves initializing instance feature buffers for all the
    /// feature types associated with the model if they do not already exist.
    ///
    /// # Panics
    /// - If the model's material is not present in the material library.
    /// - If any of the model's feature types have not been registered with
    ///   [`register_feature_type`].
    pub fn register_instance(&mut self, material_library: &MaterialLibrary, model_id: ModelID)
    where
        InstanceModelViewTransform: InstanceFeature,
    {
        let mut feature_type_ids = Vec::with_capacity(2);

        feature_type_ids.extend_from_slice(
            material_library
                .get_material_specification(model_id.material_handle().material_id())
                .expect("Missing material specification for model material")
                .instance_feature_type_ids(),
        );

        if let Some(prepass_material_handle) = model_id.prepass_material_handle() {
            feature_type_ids.extend_from_slice(
                material_library
                    .get_material_specification(prepass_material_handle.material_id())
                    .expect("Missing material specification for model prepass material")
                    .instance_feature_type_ids(),
            );
        }

        self.register_instance_with_feature_type_ids(model_id, &feature_type_ids);
    }

    /// Informs the manager that an instance of the model with the given ID has
    /// been deleted, so that the associated instance feature buffers can be
    /// deleted if this was the last instance of that model.
    ///
    /// # Panics
    /// If no instance of the specified model exists.
    pub fn unregister_instance(&mut self, model_id: ModelID) {
        let count = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to unregister instance of model that has no instances")
            .0;

        assert!(*count > 0);

        *count -= 1;

        if *count == 0 {
            self.instance_feature_buffers.remove(&model_id);
        }
    }

    /// Finds the instance feature buffers for the model with the given ID and
    /// pushes the given transform and the feature values corrsponding to the
    /// given feature IDs onto the buffers.
    ///
    /// # Panics
    /// - If no buffers exist for the model with the given ID.
    /// - If the number of feature IDs is not exactly one less than the number
    ///   of buffers (the first buffer is used for the transform).
    /// - If any of the feature IDs are for feature types other than the type
    ///   stored in the corresponding buffer (the order of the feature IDs has
    ///   to be the same as in the
    ///   [`MaterialSpecification`](crate::scene::MaterialSpecification) of the
    ///   model, which was used to initialize the buffers.
    pub fn buffer_instance(
        &mut self,
        model_id: ModelID,
        transform: &InstanceModelViewTransform,
        feature_ids: &[InstanceFeatureID],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instance of missing model")
            .1;

        assert_eq!(feature_ids.len() + 1, feature_buffers.len());

        let mut feature_buffers = feature_buffers.iter_mut();

        feature_buffers
            .next()
            .expect("Missing transform buffer for instance")
            .add_feature(transform);

        for (&feature_id, feature_buffer) in feature_ids.iter().zip(feature_buffers) {
            let feature_type_id = feature_buffer.feature_type_id();

            let storage = self
                .feature_storages
                .get(&feature_type_id)
                .expect("Missing storage for model instance feature");

            feature_buffer.add_feature_from_storage(storage, feature_id);
        }
    }

    /// Finds the instance feature buffers for the model with the given ID and
    /// pushes the given set of transforms (one for each instance) and the
    /// feature values corrsponding to the given sets of feature IDs (one
    /// [`Vec`] for each feature type, each [`Vec`] containing the feature ID of
    /// each instance) onto the buffers.
    ///
    /// # Panics
    /// - If no buffers exist for the model with the given ID.
    /// - If the number of feature IDs is not exactly one less than the number
    ///   of buffers (the first buffer is used for the transform).
    /// - If any of the feature IDs are for feature types other than the type
    ///   stored in the corresponding buffer (the order of the feature IDs has
    ///   to be the same as in the
    ///   [`MaterialSpecification`](crate::scene::MaterialSpecification) of the
    ///   model, which was used to initialize the buffers.
    /// - If any of the [`Vec`] with feature IDs has a different length than the
    ///   slice of transforms.
    pub fn buffer_multiple_instances(
        &mut self,
        model_id: ModelID,
        transforms: &[InstanceModelViewTransform],
        feature_ids: &[Vec<InstanceFeatureID>],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instances of missing model")
            .1;

        assert_eq!(feature_ids.len() + 1, feature_buffers.len());

        let mut feature_buffers = feature_buffers.iter_mut();

        let transform_buffer = feature_buffers
            .next()
            .expect("Missing transform buffer for instance");

        transform_buffer.add_feature_slice(transforms);
        let n_instances = transforms.len();

        for (feature_ids, feature_buffer) in feature_ids.iter().zip(feature_buffers) {
            assert_eq!(
                feature_ids.len(),
                n_instances,
                "Encountered different instance counts for different feature types when buffering multiple instances"
            );

            let feature_type_id = feature_buffer.feature_type_id();

            let storage = self
                .feature_storages
                .get(&feature_type_id)
                .expect("Missing storage for model instance feature");

            for &feature_id in feature_ids {
                feature_buffer.add_feature_from_storage(storage, feature_id);
            }
        }
    }

    /// Finds the instance transform buffer for the model with the given ID and
    /// pushes the given transfrom onto it.
    ///
    /// # Panics
    /// If no buffers exist for the model with the given ID.
    pub fn buffer_instance_transform(
        &mut self,
        model_id: ModelID,
        transform: &InstanceModelViewTransform,
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instance of missing model")
            .1;

        feature_buffers
            .get_mut(0)
            .expect("Missing transform buffer for instance")
            .add_feature(transform);
    }

    /// Finds the instance transform buffer for the model with the given ID and
    /// pushes the given transforms onto it.
    ///
    /// # Panics
    /// If no buffers exist for the model with the given ID.
    pub fn buffer_multiple_instance_transforms(
        &mut self,
        model_id: ModelID,
        transforms: &[InstanceModelViewTransform],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        let feature_buffers = &mut self
            .instance_feature_buffers
            .get_mut(&model_id)
            .expect("Tried to buffer instance of missing model")
            .1;

        feature_buffers
            .get_mut(0)
            .expect("Missing transform buffer for instance")
            .add_feature_slice(transforms);
    }

    fn register_instance_with_feature_type_ids(
        &mut self,
        model_id: ModelID,
        feature_type_ids: &[InstanceFeatureTypeID],
    ) where
        InstanceModelViewTransform: InstanceFeature,
    {
        self.instance_feature_buffers
            .entry(model_id)
            .and_modify(|(count, buffers)| {
                assert_eq!(buffers.len(), feature_type_ids.len() + 1);
                *count += 1;
            })
            .or_insert_with(|| {
                let mut buffers = Vec::with_capacity(feature_type_ids.len() + 1);
                buffers.push(DynamicInstanceFeatureBuffer::new::<
                    InstanceModelViewTransform,
                >());
                buffers.extend(feature_type_ids.iter().map(|feature_type_id| {
                    let storage = self.feature_storages.get(feature_type_id).expect(
                        "Missing storage for instance feature type \
                             (all feature types must be registered with `register_feature_type`)",
                    );
                    DynamicInstanceFeatureBuffer::new_for_storage(storage)
                }));
                (1, buffers)
            });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        impl_InstanceFeature,
        rendering::InstanceFeatureShaderInput,
        scene::{MaterialHandle, MaterialID, MeshID},
    };
    use bytemuck::{Pod, Zeroable};
    use impact_utils::hash64;
    use nalgebra::{Similarity3, Translation3, UnitQuaternion};

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
    struct Feature(u8);

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    struct DifferentFeature(f64);

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
    struct ZeroSizedFeature;

    impl_InstanceFeature!(Feature, [], InstanceFeatureShaderInput::None);
    impl_InstanceFeature!(DifferentFeature, [], InstanceFeatureShaderInput::None);
    impl_InstanceFeature!(ZeroSizedFeature, [], InstanceFeatureShaderInput::None);

    fn create_dummy_model_id<S: AsRef<str>>(tag: S) -> ModelID {
        ModelID::for_mesh_and_material(
            MeshID(hash64!(format!("Test mesh {}", tag.as_ref()))),
            MaterialHandle::new(
                MaterialID(hash64!(format!("Test material {}", tag.as_ref()))),
                None,
                None,
            ),
            None,
        )
    }

    fn create_dummy_transform() -> InstanceModelViewTransform {
        InstanceModelViewTransform::with_model_view_transform(Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3),
            7.0,
        ))
    }

    fn create_dummy_transform_2() -> InstanceModelViewTransform {
        InstanceModelViewTransform::with_model_view_transform(Similarity3::from_parts(
            Translation3::new(6.1, -2.7, -0.21),
            UnitQuaternion::from_euler_angles(1.1, 3.2, 2.3),
            3.0,
        ))
    }

    #[test]
    fn creating_instance_feature_manager_works() {
        let manager = InstanceFeatureManager::new();
        assert!(manager.model_ids_and_buffers().next().is_none());
    }

    #[test]
    fn registering_feature_types_for_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();

        assert!(manager.get_storage::<ZeroSizedFeature>().is_none());
        assert!(manager.get_storage_mut::<ZeroSizedFeature>().is_none());

        manager.register_feature_type::<ZeroSizedFeature>();

        let storage_1 = manager.get_storage::<ZeroSizedFeature>().unwrap();
        assert_eq!(
            storage_1.feature_type_id(),
            ZeroSizedFeature::FEATURE_TYPE_ID
        );
        assert_eq!(storage_1.feature_count(), 0);

        assert!(manager.get_storage::<Feature>().is_none());
        assert!(manager.get_storage_mut::<Feature>().is_none());

        manager.register_feature_type::<Feature>();

        let storage_2 = manager.get_storage::<Feature>().unwrap();
        assert_eq!(storage_2.feature_type_id(), Feature::FEATURE_TYPE_ID);
        assert_eq!(storage_2.feature_count(), 0);
    }

    #[test]
    fn registering_one_instance_of_one_model_with_no_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);

        let mut models_and_buffers = manager.model_ids_and_buffers();
        let (registered_model_id, buffers) = models_and_buffers.next().unwrap();
        assert_eq!(registered_model_id, model_id);
        assert_eq!(buffers.len(), 1);
        assert_eq!(
            buffers[0].feature_type_id(),
            InstanceModelViewTransform::FEATURE_TYPE_ID
        );
        assert_eq!(buffers[0].n_valid_bytes(), 0);

        assert!(models_and_buffers.next().is_none());
        drop(models_and_buffers);

        assert!(manager.has_model_id(model_id));
        let buffers = manager.get_buffers(model_id).unwrap();
        assert_eq!(buffers.len(), 1);
        assert_eq!(
            buffers[0].feature_type_id(),
            InstanceModelViewTransform::FEATURE_TYPE_ID
        );
        assert_eq!(buffers[0].n_valid_bytes(), 0);
    }

    #[test]
    #[should_panic]
    fn registering_instance_with_unregistered_features_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);
    }

    #[test]
    fn registering_one_instance_of_one_model_with_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();
        manager.register_feature_type::<Feature>();
        manager.register_feature_type::<ZeroSizedFeature>();

        let model_id = create_dummy_model_id("");

        manager.register_instance_with_feature_type_ids(
            model_id,
            &[ZeroSizedFeature::FEATURE_TYPE_ID, Feature::FEATURE_TYPE_ID],
        );

        assert!(manager.has_model_id(model_id));
        let buffers = manager.get_buffers(model_id).unwrap();
        assert_eq!(buffers.len(), 3);
        assert_eq!(
            buffers[0].feature_type_id(),
            InstanceModelViewTransform::FEATURE_TYPE_ID
        );
        assert_eq!(buffers[0].n_valid_bytes(), 0);
        assert_eq!(
            buffers[1].feature_type_id(),
            ZeroSizedFeature::FEATURE_TYPE_ID
        );
        assert_eq!(buffers[1].n_valid_bytes(), 0);
        assert_eq!(buffers[2].feature_type_id(), Feature::FEATURE_TYPE_ID);
        assert_eq!(buffers[2].n_valid_bytes(), 0);
    }

    #[test]
    fn registering_one_instance_of_two_models_with_no_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();
        let model_id_1 = create_dummy_model_id("1");
        let model_id_2 = create_dummy_model_id("2");
        manager.register_instance_with_feature_type_ids(model_id_1, &[]);
        manager.register_instance_with_feature_type_ids(model_id_2, &[]);

        let mut models_and_buffers = manager.model_ids_and_buffers();
        assert_ne!(
            models_and_buffers.next().unwrap().0,
            models_and_buffers.next().unwrap().0
        );
        assert!(models_and_buffers.next().is_none());
        drop(models_and_buffers);

        assert!(manager.has_model_id(model_id_1));
        assert!(manager.has_model_id(model_id_2));
        assert_eq!(manager.get_buffers(model_id_1).unwrap().len(), 1);
        assert_eq!(manager.get_buffers(model_id_2).unwrap().len(), 1);
    }

    #[test]
    fn registering_two_instances_of_one_model_with_no_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);
        manager.register_instance_with_feature_type_ids(model_id, &[]);

        let mut models_and_buffers = manager.model_ids_and_buffers();
        assert_eq!(models_and_buffers.next().unwrap().0, model_id);
        assert!(models_and_buffers.next().is_none());
        drop(models_and_buffers);

        assert!(manager.has_model_id(model_id));
        assert_eq!(manager.get_buffers(model_id).unwrap().len(), 1);
    }

    #[test]
    fn registering_and_then_unregistering_one_instance_of_model_in_instance_feature_manager_works()
    {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");

        manager.register_instance_with_feature_type_ids(model_id, &[]);
        manager.unregister_instance(model_id);

        assert!(manager.model_ids_and_buffers().next().is_none());
        assert!(!manager.has_model_id(model_id));
        assert!(manager.get_buffers(model_id).is_none());

        manager.register_instance_with_feature_type_ids(model_id, &[]);
        manager.unregister_instance(model_id);

        assert!(manager.model_ids_and_buffers().next().is_none());
        assert!(!manager.has_model_id(model_id));
        assert!(manager.get_buffers(model_id).is_none());
    }

    #[test]
    fn registering_and_then_unregistering_two_instances_of_model_in_instance_feature_manager_works()
    {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);
        manager.register_instance_with_feature_type_ids(model_id, &[]);
        manager.unregister_instance(model_id);
        manager.unregister_instance(model_id);

        assert!(manager.model_ids_and_buffers().next().is_none());
        assert!(!manager.has_model_id(model_id));
        assert!(manager.get_buffers(model_id).is_none());
    }

    #[test]
    #[should_panic]
    fn unregistering_instance_in_empty_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.unregister_instance(model_id);
    }

    #[test]
    #[should_panic]
    fn buffering_unregistered_instance_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[]);
    }

    #[test]
    #[should_panic]
    fn buffering_multiple_of_unregistered_instance_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.buffer_multiple_instances(
            model_id,
            &[InstanceModelViewTransform::identity(); 2],
            &[],
        );
    }

    #[test]
    fn buffering_instances_with_no_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);

        let transform_1 = create_dummy_transform();
        let transform_2 = InstanceModelViewTransform::identity();

        manager.buffer_instance(model_id, &transform_1, &[]);

        let buffer = &manager.get_buffers(model_id).unwrap()[0];
        assert_eq!(buffer.n_valid_features(), 1);
        assert_eq!(
            buffer.valid_features::<InstanceModelViewTransform>(),
            &[transform_1]
        );

        manager.buffer_instance(model_id, &transform_2, &[]);

        let buffer = &manager.get_buffers(model_id).unwrap()[0];
        assert_eq!(buffer.n_valid_features(), 2);
        assert_eq!(
            buffer.valid_features::<InstanceModelViewTransform>(),
            &[transform_1, transform_2]
        );
    }

    #[test]
    fn buffering_multiple_instances_with_no_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);

        let transform_1 = create_dummy_transform();
        let transform_2 = create_dummy_transform_2();
        let transform_3 = InstanceModelViewTransform::identity();

        manager.buffer_multiple_instances(model_id, &[transform_1, transform_2], &[]);

        let buffer = &manager.get_buffers(model_id).unwrap()[0];
        assert_eq!(buffer.n_valid_features(), 2);
        assert_eq!(
            buffer.valid_features::<InstanceModelViewTransform>(),
            &[transform_1, transform_2]
        );

        manager.buffer_multiple_instances(model_id, &[transform_3], &[]);

        let buffer = &manager.get_buffers(model_id).unwrap()[0];
        assert_eq!(buffer.n_valid_features(), 3);
        assert_eq!(
            buffer.valid_features::<InstanceModelViewTransform>(),
            &[transform_1, transform_2, transform_3]
        );
    }

    #[test]
    fn buffering_instance_with_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();

        manager.register_feature_type::<Feature>();
        manager.register_feature_type::<DifferentFeature>();

        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(
            model_id,
            &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
        );

        let transform_instance_1 = create_dummy_transform();
        let transform_instance_2 = InstanceModelViewTransform::identity();
        let feature_1_instance_1 = Feature(22);
        let feature_1_instance_2 = Feature(43);
        let feature_2_instance_1 = DifferentFeature(-73.1);
        let feature_2_instance_2 = DifferentFeature(32.7);

        let id_1_instance_1 = manager
            .get_storage_mut::<Feature>()
            .unwrap()
            .add_feature(&feature_1_instance_1);
        let id_2_instance_1 = manager
            .get_storage_mut::<DifferentFeature>()
            .unwrap()
            .add_feature(&feature_2_instance_1);
        let id_1_instance_2 = manager
            .get_storage_mut::<Feature>()
            .unwrap()
            .add_feature(&feature_1_instance_2);
        let id_2_instance_2 = manager
            .get_storage_mut::<DifferentFeature>()
            .unwrap()
            .add_feature(&feature_2_instance_2);

        manager.buffer_instance(
            model_id,
            &transform_instance_1,
            &[id_1_instance_1, id_2_instance_1],
        );

        let buffers = manager.get_buffers(model_id).unwrap();
        assert_eq!(buffers.len(), 3);
        assert_eq!(buffers[0].n_valid_features(), 1);
        assert_eq!(
            buffers[0].valid_features::<InstanceModelViewTransform>(),
            &[transform_instance_1]
        );
        assert_eq!(buffers[1].n_valid_features(), 1);
        assert_eq!(
            buffers[1].valid_features::<Feature>(),
            &[feature_1_instance_1]
        );
        assert_eq!(buffers[2].n_valid_features(), 1);
        assert_eq!(
            buffers[2].valid_features::<DifferentFeature>(),
            &[feature_2_instance_1]
        );

        manager.buffer_instance(
            model_id,
            &transform_instance_2,
            &[id_1_instance_2, id_2_instance_2],
        );

        let buffers = manager.get_buffers(model_id).unwrap();
        assert_eq!(buffers.len(), 3);
        assert_eq!(buffers[0].n_valid_features(), 2);
        assert_eq!(
            buffers[0].valid_features::<InstanceModelViewTransform>(),
            &[transform_instance_1, transform_instance_2]
        );
        assert_eq!(buffers[1].n_valid_features(), 2);
        assert_eq!(
            buffers[1].valid_features::<Feature>(),
            &[feature_1_instance_1, feature_1_instance_2]
        );
        assert_eq!(buffers[2].n_valid_features(), 2);
        assert_eq!(
            buffers[2].valid_features::<DifferentFeature>(),
            &[feature_2_instance_1, feature_2_instance_2]
        );
    }

    #[test]
    fn buffering_multiple_instances_with_features_in_instance_feature_manager_works() {
        let mut manager = InstanceFeatureManager::new();

        manager.register_feature_type::<Feature>();
        manager.register_feature_type::<DifferentFeature>();

        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(
            model_id,
            &[Feature::FEATURE_TYPE_ID, DifferentFeature::FEATURE_TYPE_ID],
        );

        let transform_instance_1 = create_dummy_transform();
        let transform_instance_2 = create_dummy_transform_2();
        let transform_instance_3 = InstanceModelViewTransform::identity();
        let feature_1_instance_1 = Feature(22);
        let feature_1_instance_2 = Feature(43);
        let feature_1_instance_3 = Feature(31);
        let feature_2_instance_1 = DifferentFeature(-73.1);
        let feature_2_instance_2 = DifferentFeature(32.7);
        let feature_2_instance_3 = DifferentFeature(2.72);

        let id_1_instance_1 = manager
            .get_storage_mut::<Feature>()
            .unwrap()
            .add_feature(&feature_1_instance_1);
        let id_2_instance_1 = manager
            .get_storage_mut::<DifferentFeature>()
            .unwrap()
            .add_feature(&feature_2_instance_1);
        let id_1_instance_2 = manager
            .get_storage_mut::<Feature>()
            .unwrap()
            .add_feature(&feature_1_instance_2);
        let id_2_instance_2 = manager
            .get_storage_mut::<DifferentFeature>()
            .unwrap()
            .add_feature(&feature_2_instance_2);
        let id_1_instance_3 = manager
            .get_storage_mut::<Feature>()
            .unwrap()
            .add_feature(&feature_1_instance_3);
        let id_2_instance_3 = manager
            .get_storage_mut::<DifferentFeature>()
            .unwrap()
            .add_feature(&feature_2_instance_3);

        manager.buffer_multiple_instances(
            model_id,
            &[transform_instance_1, transform_instance_2],
            &[
                vec![id_1_instance_1, id_1_instance_2],
                vec![id_2_instance_1, id_2_instance_2],
            ],
        );

        let buffers = manager.get_buffers(model_id).unwrap();
        assert_eq!(buffers.len(), 3);
        assert_eq!(buffers[0].n_valid_features(), 2);
        assert_eq!(
            buffers[0].valid_features::<InstanceModelViewTransform>(),
            &[transform_instance_1, transform_instance_2]
        );
        assert_eq!(buffers[1].n_valid_features(), 2);
        assert_eq!(
            buffers[1].valid_features::<Feature>(),
            &[feature_1_instance_1, feature_1_instance_2]
        );
        assert_eq!(buffers[2].n_valid_features(), 2);
        assert_eq!(
            buffers[2].valid_features::<DifferentFeature>(),
            &[feature_2_instance_1, feature_2_instance_2]
        );

        manager.buffer_multiple_instances(
            model_id,
            &[transform_instance_3],
            &[vec![id_1_instance_3], vec![id_2_instance_3]],
        );

        let buffers = manager.get_buffers(model_id).unwrap();
        assert_eq!(buffers.len(), 3);
        assert_eq!(buffers[0].n_valid_features(), 3);
        assert_eq!(
            buffers[0].valid_features::<InstanceModelViewTransform>(),
            &[
                transform_instance_1,
                transform_instance_2,
                transform_instance_3
            ]
        );
        assert_eq!(buffers[1].n_valid_features(), 3);
        assert_eq!(
            buffers[1].valid_features::<Feature>(),
            &[
                feature_1_instance_1,
                feature_1_instance_2,
                feature_1_instance_3
            ]
        );
        assert_eq!(buffers[2].n_valid_features(), 3);
        assert_eq!(
            buffers[2].valid_features::<DifferentFeature>(),
            &[
                feature_2_instance_1,
                feature_2_instance_2,
                feature_2_instance_3
            ]
        );
    }

    #[test]
    #[should_panic]
    fn buffering_instance_with_too_many_feature_ids_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);

        let mut storage = InstanceFeatureStorage::new::<Feature>();
        let id = storage.add_feature(&Feature(33));

        manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[id]);
    }

    #[test]
    #[should_panic]
    fn buffering_multiple_instances_with_too_many_feature_ids_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[]);

        let mut storage = InstanceFeatureStorage::new::<Feature>();
        let id = storage.add_feature(&Feature(33));

        manager.buffer_multiple_instances(
            model_id,
            &[InstanceModelViewTransform::identity()],
            &[vec![id]],
        );
    }

    #[test]
    #[should_panic]
    fn buffering_instance_with_too_few_feature_ids_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        manager.register_feature_type::<Feature>();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);
        manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[]);
    }

    #[test]
    #[should_panic]
    fn buffering_multiple_instances_with_too_few_feature_ids_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        manager.register_feature_type::<Feature>();
        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);
        manager.buffer_multiple_instances(model_id, &[InstanceModelViewTransform::identity()], &[]);
    }

    #[test]
    #[should_panic]
    fn buffering_instance_with_invalid_feature_id_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        manager.register_feature_type::<Feature>();

        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);

        let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
        let id = storage.add_feature(&DifferentFeature(-0.2));

        manager.buffer_instance(model_id, &InstanceModelViewTransform::identity(), &[id]);
    }

    #[test]
    #[should_panic]
    fn buffering_multiple_instances_with_invalid_feature_id_in_instance_feature_manager_fails() {
        let mut manager = InstanceFeatureManager::new();
        manager.register_feature_type::<Feature>();

        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);

        let mut storage = InstanceFeatureStorage::new::<DifferentFeature>();
        let id = storage.add_feature(&DifferentFeature(-0.2));

        manager.buffer_multiple_instances(
            model_id,
            &[InstanceModelViewTransform::identity()],
            &[vec![id]],
        );
    }

    #[test]
    #[should_panic]
    fn buffering_multiple_instances_with_different_transform_and_feature_id_counts_in_instance_feature_manager_fails(
    ) {
        let mut manager = InstanceFeatureManager::new();
        manager.register_feature_type::<Feature>();

        let model_id = create_dummy_model_id("");
        manager.register_instance_with_feature_type_ids(model_id, &[Feature::FEATURE_TYPE_ID]);

        let mut storage = InstanceFeatureStorage::new::<Feature>();
        let id = storage.add_feature(&Feature(42));

        manager.buffer_multiple_instances(
            model_id,
            &[
                create_dummy_transform(),
                InstanceModelViewTransform::identity(),
            ],
            &[vec![id]],
        );
    }
}
