use impact::egui::ComboBox;
use impact::egui::{DragValue, Response, Ui, emath::Numeric};
use impact::impact_math::Hash64;
use impact_dev_ui::option_panels::{LabelAndHoverText, labeled_option, option_drag_value};
use impact_voxel::generation::sdf::meta::params::{self as core, ParamIdx};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tinyvec::TinyVec;

#[derive(Clone, Debug)]
pub struct MetaNodeParams {
    pub params: ParamList<MetaNodeParam>,
    pub param_names: ParamList<&'static str>,
}

type ParamList<T> = TinyVec<[T; 12]>;

#[derive(Clone, Debug)]
pub enum MetaNodeParam {
    Enum(MetaEnumParam),
    UInt(MetaUIntParam),
    Float(MetaFloatParam),
    Distributed(MetaDistributedParam),
}

#[derive(Clone, Debug)]
pub struct MetaEnumParam {
    pub text: LabelAndHoverText,
    pub variants: EnumParamVariants,
    pub value: &'static str,
}

pub type EnumParamVariants = TinyVec<[&'static str; 2]>;

#[derive(Clone, Debug)]
pub struct MetaUIntParam {
    pub text: LabelAndHoverText,
    pub value: u32,
    pub speed: f32,
}

#[derive(Clone, Debug)]
pub struct MetaFloatParam {
    pub text: LabelAndHoverText,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub speed: f32,
}

#[derive(Clone, Debug)]
pub struct MetaDistributedParam {
    pub text: LabelAndHoverText,
    pub value_type: ParamValueType,
    pub distribution: ParamDistribution,
    pub min_value: f32,
    pub max_value: f32,
    pub speed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamValueType {
    Discrete,
    Continuous,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParamDistribution {
    pub variant: DistributionVariant,
    pub constant: ValueSource,
    pub random_uniform: RandomUniformDistribution,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RandomUniformDistribution {
    pub min: ValueSource,
    pub max: ValueSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionVariant {
    #[default]
    Constant,
    RandomUniform,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValueSource {
    pub variant: ValueSourceVariant,
    pub fixed: f32,
    pub from_param: FromParamSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueSourceVariant {
    #[default]
    Fixed,
    FromParam,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FromParamSource {
    pub param_idx: ParamIdx,
    pub mapping: ParamValueMapping,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParamValueMapping {
    pub variant: ParamValueMappingVariant,
    pub linear: LinearParamValueMapping,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamValueMappingVariant {
    #[default]
    Linear,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinearParamValueMapping {
    pub offset: f32,
    pub scale: f32,
}

impl MetaNodeParams {
    pub fn new() -> Self {
        Self {
            params: ParamList::new(),
            param_names: ParamList::new(),
        }
    }

    pub fn push(&mut self, param: impl Into<MetaNodeParam>) {
        let param = param.into();
        self.param_names.push(param.name());
        self.params.push(param);
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }
}

impl MetaNodeParam {
    pub fn show_controls_and_return_changed(
        &mut self,
        ui: &mut Ui,
        current_param_idx: ParamIdx,
        param_names: &[&'static str],
    ) -> bool {
        match self {
            Self::Enum(param) => param.show_controls_and_return_changed(ui),
            Self::UInt(param) => param.show_controls(ui).changed(),
            Self::Float(param) => param.show_controls(ui).changed(),
            Self::Distributed(param) => {
                param.show_controls_and_return_changed(ui, current_param_idx, param_names)
            }
        }
    }

    pub fn text_to_display(&self) -> String {
        match self {
            Self::Enum(param) => param.text_to_display(),
            Self::UInt(param) => param.text_to_display(),
            Self::Float(param) => param.text_to_display(),
            Self::Distributed(param) => param.text_to_display(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Enum(param) => param.text.label,
            Self::UInt(param) => param.text.label,
            Self::Float(param) => param.text.label,
            Self::Distributed(param) => param.text.label,
        }
    }

    pub fn as_enum_value(&self) -> Option<&'static str> {
        if let Self::Enum(param) = self {
            Some(param.value)
        } else {
            None
        }
    }

    pub fn enum_value(&self) -> &'static str {
        self.as_enum_value().unwrap()
    }

    pub fn as_uint(&self) -> Option<u32> {
        if let Self::UInt(param) = self {
            Some(param.value)
        } else {
            None
        }
    }

    pub fn uint(&self) -> u32 {
        self.as_uint().unwrap()
    }

    pub fn as_float(&self) -> Option<f32> {
        if let Self::Float(param) = self {
            Some(param.value)
        } else {
            None
        }
    }

    pub fn float(&self) -> f32 {
        self.as_float().unwrap()
    }

    pub fn as_discrete_spec(&self) -> Option<core::DiscreteParamSpec> {
        if let Self::Distributed(param) = self {
            param.as_discrete_spec()
        } else {
            None
        }
    }

    pub fn discrete_spec(&self) -> core::DiscreteParamSpec {
        self.as_discrete_spec().unwrap()
    }

    pub fn as_cont_spec(&self) -> Option<core::ContParamSpec> {
        if let Self::Distributed(param) = self {
            param.as_cont_spec()
        } else {
            None
        }
    }

    pub fn cont_spec(&self) -> core::ContParamSpec {
        self.as_cont_spec().unwrap()
    }
}

impl<'a> From<&'a MetaNodeParam> for u32 {
    fn from(param: &'a MetaNodeParam) -> Self {
        param.uint()
    }
}

impl<'a> From<&'a MetaNodeParam> for f32 {
    fn from(param: &'a MetaNodeParam) -> Self {
        param.float()
    }
}

impl<'a> From<&'a MetaNodeParam> for core::DiscreteParamSpec {
    fn from(param: &'a MetaNodeParam) -> Self {
        param.discrete_spec()
    }
}

impl<'a> From<&'a MetaNodeParam> for core::ContParamSpec {
    fn from(param: &'a MetaNodeParam) -> Self {
        param.cont_spec()
    }
}

impl From<MetaEnumParam> for MetaNodeParam {
    fn from(param: MetaEnumParam) -> Self {
        Self::Enum(param)
    }
}

impl From<MetaUIntParam> for MetaNodeParam {
    fn from(param: MetaUIntParam) -> Self {
        Self::UInt(param)
    }
}

impl From<MetaFloatParam> for MetaNodeParam {
    fn from(param: MetaFloatParam) -> Self {
        Self::Float(param)
    }
}

impl From<MetaDistributedParam> for MetaNodeParam {
    fn from(param: MetaDistributedParam) -> Self {
        Self::Distributed(param)
    }
}

impl Default for MetaNodeParam {
    fn default() -> Self {
        Self::UInt(MetaUIntParam {
            text: LabelAndHoverText::label_only(""),
            value: 0,
            speed: 0.0,
        })
    }
}

impl MetaEnumParam {
    pub fn new(text: LabelAndHoverText, variants: EnumParamVariants, value: &'static str) -> Self {
        assert!(variants.contains(&value));
        Self {
            text,
            variants,
            value,
        }
    }

    fn show_controls_and_return_changed(&mut self, ui: &mut Ui) -> bool {
        let old_value_hash = Hash64::from_str(self.value);
        labeled_option(ui, self.text.clone(), |ui| {
            ComboBox::from_id_salt(("meta_enum_param", self.text.label))
                .selected_text(self.value)
                .show_ui(ui, |ui| {
                    for &variant in &self.variants {
                        ui.selectable_value(&mut self.value, variant, variant);
                    }
                })
        });
        old_value_hash != Hash64::from_str(self.value)
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

impl MetaUIntParam {
    pub const fn new(text: LabelAndHoverText, value: u32) -> Self {
        Self {
            text,
            value,
            speed: 0.05,
        }
    }

    fn show_controls(&mut self, ui: &mut Ui) -> Response {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value)
                .fixed_decimals(0)
                .speed(self.speed),
        )
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

impl MetaFloatParam {
    pub const fn new(text: LabelAndHoverText, value: f32) -> Self {
        Self {
            text,
            value,
            min_value: f32::NEG_INFINITY,
            max_value: f32::INFINITY,
            speed: 0.05,
        }
    }

    pub const fn with_min_value(mut self, min_value: f32) -> Self {
        self.min_value = min_value;
        self
    }

    pub const fn with_max_value(mut self, max_value: f32) -> Self {
        self.max_value = max_value;
        self
    }

    pub const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    fn show_controls(&mut self, ui: &mut Ui) -> Response {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value)
                .range(self.min_value..=self.max_value)
                .speed(self.speed),
        )
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

impl MetaDistributedParam {
    pub fn new_fixed_constant_discrete_value(text: LabelAndHoverText, value: u32) -> Self {
        Self {
            text,
            value_type: ParamValueType::Discrete,
            distribution: ParamDistribution::fixed_constant(value as f32),
            min_value: 0.0,
            max_value: u32::MAX as f32,
            speed: 0.05,
        }
    }

    pub fn new_fixed_constant_continuous_value(text: LabelAndHoverText, value: f32) -> Self {
        Self {
            text,
            value_type: ParamValueType::Continuous,
            distribution: ParamDistribution::fixed_constant(value),
            min_value: f32::NEG_INFINITY,
            max_value: f32::INFINITY,
            speed: 0.05,
        }
    }

    pub fn with_min_value(mut self, min_value: f32) -> Self {
        assert_eq!(self.value_type, ParamValueType::Continuous);
        self.min_value = min_value;
        self
    }

    pub fn with_max_value(mut self, max_value: f32) -> Self {
        assert_eq!(self.value_type, ParamValueType::Continuous);
        self.max_value = max_value;
        self
    }

    pub const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    fn show_controls_and_return_changed(
        &mut self,
        ui: &mut Ui,
        current_param_idx: ParamIdx,
        param_names: &[&'static str],
    ) -> bool {
        let mut changed = false;

        let id_salt = ("distributed_param", self.text.label);
        let distribution = &mut self.distribution;

        distribution_combobox(ui, id_salt, &mut distribution.variant, &mut changed);

        match distribution.variant {
            DistributionVariant::Constant => {
                let id_salt = (id_salt, "constant");
                source_controls(
                    ui,
                    id_salt,
                    &mut distribution.constant,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_param_idx,
                    param_names,
                    &mut changed,
                );
            }
            DistributionVariant::RandomUniform => {
                let id_salt = (id_salt, "random_uniform");
                let random_uniform = &mut distribution.random_uniform;
                source_controls(
                    ui,
                    (id_salt, "min"),
                    &mut random_uniform.min,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_param_idx,
                    param_names,
                    &mut changed,
                );

                source_controls(
                    ui,
                    (id_salt, "max"),
                    &mut random_uniform.min,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_param_idx,
                    param_names,
                    &mut changed,
                );
            }
        }

        changed
    }

    fn text_to_display(&self) -> String {
        format!("{self:?}")
    }

    fn as_discrete_spec(&self) -> Option<core::DiscreteParamSpec> {
        (self.value_type == ParamValueType::Discrete)
            .then(|| self.distribution.to_discrete_param_spec())
    }

    fn as_cont_spec(&self) -> Option<core::ContParamSpec> {
        (self.value_type == ParamValueType::Continuous)
            .then(|| self.distribution.to_cont_param_spec())
    }
}

impl ParamDistribution {
    fn fixed_constant(value: f32) -> Self {
        Self {
            variant: DistributionVariant::Constant,
            constant: ValueSource::fixed(value),
            ..Self::default()
        }
    }

    fn to_discrete_param_spec(&self) -> core::DiscreteParamSpec {
        match self.variant {
            DistributionVariant::Constant => {
                core::DiscreteParamSpec::Constant((&self.constant).into())
            }
            DistributionVariant::RandomUniform => core::DiscreteParamSpec::RandomUniform {
                min: (&self.random_uniform.min).into(),
                max: (&self.random_uniform.max).into(),
            },
        }
    }

    fn to_cont_param_spec(&self) -> core::ContParamSpec {
        match self.variant {
            DistributionVariant::Constant => core::ContParamSpec::Constant((&self.constant).into()),
            DistributionVariant::RandomUniform => core::ContParamSpec::RandomUniform {
                min: (&self.random_uniform.min).into(),
                max: (&self.random_uniform.max).into(),
            },
        }
    }
}

impl ValueSource {
    fn fixed(value: f32) -> Self {
        Self {
            variant: ValueSourceVariant::Fixed,
            fixed: value,
            ..Self::default()
        }
    }
}

impl<'a> From<&'a ValueSource> for core::DiscreteValueSource {
    fn from(source: &'a ValueSource) -> Self {
        match source.variant {
            ValueSourceVariant::Fixed => core::DiscreteValueSource::Fixed(source.fixed as u32),
            ValueSourceVariant::FromParam => core::DiscreteValueSource::FromParam {
                idx: source.from_param.param_idx,
                mapping: (&source.from_param.mapping).into(),
            },
        }
    }
}

impl<'a> From<&'a ValueSource> for core::ContValueSource {
    fn from(source: &'a ValueSource) -> Self {
        match source.variant {
            ValueSourceVariant::Fixed => core::ContValueSource::Fixed(source.fixed),
            ValueSourceVariant::FromParam => core::ContValueSource::FromParam {
                idx: source.from_param.param_idx,
                mapping: (&source.from_param.mapping).into(),
            },
        }
    }
}

impl<'a> From<&'a ParamValueMapping> for core::ParamValueMapping {
    fn from(mapping: &'a ParamValueMapping) -> Self {
        match mapping.variant {
            ParamValueMappingVariant::Linear => core::ParamValueMapping::Linear {
                offset: mapping.linear.offset,
                scale: mapping.linear.scale,
            },
        }
    }
}

fn distribution_combobox(
    ui: &mut Ui,
    id_salt: impl Hash,
    selected_variant: &mut DistributionVariant,
    changed: &mut bool,
) {
    let variant_before = *selected_variant;
    labeled_option(ui, LabelAndHoverText::label_only("Distribution"), |ui| {
        ComboBox::from_id_salt(id_salt)
            .selected_text(selected_variant.label())
            .show_ui(ui, |ui| {
                for variant in DistributionVariant::all() {
                    ui.selectable_value(selected_variant, variant, variant.label());
                }
            })
    });
    if *selected_variant != variant_before {
        *changed = true;
    }
}

fn source_controls(
    ui: &mut Ui,
    id_salt: impl Hash + Copy,
    source: &mut ValueSource,
    value_type: ParamValueType,
    min_value: f32,
    max_value: f32,
    speed: f32,
    current_param_idx: ParamIdx,
    param_names: &[&'static str],
    changed: &mut bool,
) {
    source_combobox(ui, id_salt, &mut source.variant, changed);

    match source.variant {
        ValueSourceVariant::Fixed => {
            fixed_value_drag_values(
                ui,
                &mut source.fixed,
                value_type,
                min_value,
                max_value,
                speed,
                changed,
            );
        }
        ValueSourceVariant::FromParam => {
            let id_salt = (id_salt, "from_param");
            let from_param = &mut source.from_param;

            param_combobox(
                ui,
                id_salt,
                &mut from_param.param_idx,
                current_param_idx,
                param_names,
                changed,
            );

            let mapping = &mut from_param.mapping;

            match from_param.mapping.variant {
                ParamValueMappingVariant::Linear => {
                    linear_mapping_drag_values(ui, &mut mapping.linear, changed);
                }
            }
        }
    }
}

fn source_combobox(
    ui: &mut Ui,
    id_salt: impl Hash,
    selected_variant: &mut ValueSourceVariant,
    changed: &mut bool,
) {
    let variant_before = *selected_variant;
    labeled_option(ui, LabelAndHoverText::label_only("Source"), |ui| {
        ComboBox::from_id_salt(id_salt)
            .selected_text(selected_variant.label())
            .show_ui(ui, |ui| {
                for variant in ValueSourceVariant::all() {
                    ui.selectable_value(selected_variant, variant, variant.label());
                }
            })
    });
    if *selected_variant != variant_before {
        *changed = true;
    }
}

fn param_combobox(
    ui: &mut Ui,
    id_salt: impl Hash,
    selected_param_idx: &mut ParamIdx,
    invalid_param_idx: ParamIdx,
    param_names: &[&'static str],
    changed: &mut bool,
) {
    let param_idx_before = *selected_param_idx;
    labeled_option(ui, LabelAndHoverText::label_only("Parameter"), |ui| {
        ComboBox::from_id_salt(id_salt)
            .selected_text(param_names[*selected_param_idx as usize])
            .show_ui(ui, |ui| {
                for (idx, &name) in param_names.iter().enumerate() {
                    let idx = idx as ParamIdx;
                    if idx != invalid_param_idx {
                        ui.selectable_value(selected_param_idx, idx as ParamIdx, name);
                    }
                }
            })
    });
    if *selected_param_idx != param_idx_before {
        *changed = true;
    }
}

fn fixed_value_drag_values(
    ui: &mut Ui,
    value: &mut f32,
    value_type: ParamValueType,
    min: f32,
    max: f32,
    speed: f32,
    changed: &mut bool,
) {
    match value_type {
        ParamValueType::Continuous => {
            fixed_generic_value_drag_values(ui, value, min, max, speed, changed);
        }
        ParamValueType::Discrete => {
            fixed_discrete_value_drag_values(ui, value, min, max, speed, changed);
        }
    }
}

fn fixed_discrete_value_drag_values(
    ui: &mut Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    speed: f32,
    changed: &mut bool,
) {
    let mut discr_value = *value as u32;
    let min = min as u32;
    let max = max as u32;
    fixed_generic_value_drag_values(ui, &mut discr_value, min, max, speed, changed);
    *value = discr_value as f32;
}

fn fixed_generic_value_drag_values<Num: Numeric>(
    ui: &mut Ui,
    value: &mut Num,
    min: Num,
    max: Num,
    speed: f32,
    changed: &mut bool,
) {
    if option_drag_value(
        ui,
        LabelAndHoverText::label_only("Value"),
        DragValue::new(value).range(min..=max).speed(speed),
    )
    .changed()
    {
        *changed = true;
    }
}

fn linear_mapping_drag_values(
    ui: &mut Ui,
    linear: &mut LinearParamValueMapping,
    changed: &mut bool,
) {
    if option_drag_value(
        ui,
        LabelAndHoverText::label_only("Offset"),
        DragValue::new(&mut linear.offset),
    )
    .changed()
    {
        *changed = true;
    }
    if option_drag_value(
        ui,
        LabelAndHoverText::label_only("Scale"),
        DragValue::new(&mut linear.scale),
    )
    .changed()
    {
        *changed = true;
    }
}

impl DistributionVariant {
    fn all() -> [Self; 2] {
        [Self::Constant, Self::RandomUniform]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Constant => "Constant",
            Self::RandomUniform => "Random (uniform)",
        }
    }
}

impl ValueSourceVariant {
    fn all() -> [Self; 2] {
        [Self::Fixed, Self::FromParam]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::FromParam => "From parameter",
        }
    }
}

impl Default for LinearParamValueMapping {
    fn default() -> Self {
        Self {
            offset: 0.0,
            scale: 1.0,
        }
    }
}
