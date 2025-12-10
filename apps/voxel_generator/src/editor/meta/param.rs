use impact::egui::{ComboBox, DragValue, Response, Ui, emath::Numeric};
use impact::impact_math::hash::Hash64;
use impact_dev_ui::option_panels::{
    LabelAndHoverText, configurable_labeled_option, configurable_option_drag_value, labeled_option,
    option_drag_value, strong_option_label,
};
use impact_voxel::generation::sdf::meta::params::{self as core, ParamIdx};
use serde::{Deserialize, Serialize};
use std::{fmt::Write, hash::Hash};
use tinyvec::TinyVec;

#[derive(Clone, Debug)]
pub struct MetaNodeParams {
    pub params: ParamList<MetaNodeParam>,
    /// Only for distributed parameters.
    pub distr_param_names: ParamList<&'static str>,
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

pub type EnumParamVariants = TinyVec<[&'static str; 3]>;

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
    #[serde(default)]
    pub uniform: Option<UniformDistribution>,
    #[serde(default)]
    pub uniform_cos_angle: Option<UniformCosAngleDistribution>,
    #[serde(default)]
    pub power_law: Option<PowerLawDistribution>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UniformDistribution {
    pub min: ValueSource,
    pub max: ValueSource,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UniformCosAngleDistribution {
    pub min_angle: ValueSource,
    pub max_angle: ValueSource,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PowerLawDistribution {
    pub min: ValueSource,
    pub max: ValueSource,
    pub exponent: ValueSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionVariant {
    #[default]
    Constant,
    Uniform,
    UniformCosAngle,
    PowerLaw,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValueSource {
    pub variant: ValueSourceVariant,
    pub fixed: f32,
    #[serde(default, skip_serializing_if = "FromParamSource::is_default")]
    pub from_param: FromParamSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueSourceVariant {
    #[default]
    Fixed,
    FromParam,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FromParamSource {
    pub param_idx: ParamIdx,
    pub mapping: ParamValueMapping,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ParamValueMapping {
    pub variant: ParamValueMappingVariant,
    pub linear: LinearParamValueMapping,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamValueMappingVariant {
    #[default]
    Linear,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearParamValueMapping {
    pub offset: f32,
    pub scale: f32,
}

impl MetaNodeParams {
    pub fn new() -> Self {
        Self {
            params: ParamList::new(),
            distr_param_names: ParamList::new(),
        }
    }

    pub fn push(&mut self, param: impl Into<MetaNodeParam>) {
        let param = param.into();

        if param.is_distributed() {
            self.distr_param_names.push(param.name());
        }

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
        current_distr_param_idx: ParamIdx,
        distr_param_names: &[&'static str],
    ) -> bool {
        match self {
            Self::Enum(param) => param.show_controls_and_return_changed(ui),
            Self::UInt(param) => param.show_controls(ui).changed(),
            Self::Float(param) => param.show_controls(ui).changed(),
            Self::Distributed(param) => param.show_controls_and_return_changed(
                ui,
                current_distr_param_idx,
                distr_param_names,
            ),
        }
    }

    pub fn text_to_display(&self, distr_param_names: &[&'static str]) -> String {
        match self {
            Self::Enum(param) => param.text_to_display(),
            Self::UInt(param) => param.text_to_display(),
            Self::Float(param) => param.text_to_display(),
            Self::Distributed(param) => param.text_to_display(distr_param_names),
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

    pub fn is_distributed(&self) -> bool {
        matches!(self, Self::Distributed(_))
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
        configurable_labeled_option(
            ui,
            self.text.clone(),
            |ui| {
                ComboBox::from_id_salt(("meta_enum_param", self.text.label))
                    .selected_text(self.value)
                    .show_ui(ui, |ui| {
                        for &variant in &self.variants {
                            ui.selectable_value(&mut self.value, variant, variant);
                        }
                    })
            },
            true,
        );
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
        configurable_option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value)
                .fixed_decimals(0)
                .speed(self.speed),
            true,
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
        configurable_option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value)
                .range(self.min_value..=self.max_value)
                .speed(self.speed),
            true,
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
        current_distr_param_idx: ParamIdx,
        distr_param_names: &[&'static str],
    ) -> bool {
        let mut changed = false;

        let id_salt = ("distributed_param", self.text.label);
        let distribution = &mut self.distribution;

        strong_option_label(ui, self.text.clone());

        distribution_combobox(
            ui,
            self.value_type,
            id_salt,
            &mut distribution.variant,
            &mut changed,
        );

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
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );
            }
            DistributionVariant::Uniform => {
                let id_salt = (id_salt, "uniform");
                let uniform = distribution.get_or_init_uniform();

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Min",
                        hover_text: "The minimum random value",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "min"),
                    &mut uniform.min,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Max",
                        hover_text: "The maximum random value",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "max"),
                    &mut uniform.max,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );
            }
            DistributionVariant::UniformCosAngle => {
                let id_salt = (id_salt, "uniform_cos_angle");
                let uniform_cos_angle = distribution.get_or_init_uniform_cos_angle();

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Min angle",
                        hover_text: "The minimum random angle, in degrees",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "min_angle"),
                    &mut uniform_cos_angle.min_angle,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Max angle",
                        hover_text: "The maximum random angle, in degrees",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "max_angle"),
                    &mut uniform_cos_angle.max_angle,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );
            }
            DistributionVariant::PowerLaw => {
                let id_salt = (id_salt, "power_law");
                let power_law = distribution.get_or_init_power_law();

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Min",
                        hover_text: "The minimum random value",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "min"),
                    &mut power_law.min,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Max",
                        hover_text: "The maximum random value",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "max"),
                    &mut power_law.max,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );

                strong_option_label(
                    ui,
                    LabelAndHoverText {
                        label: "    Exponent",
                        hover_text: "The power law exponent",
                    },
                );
                source_controls(
                    ui,
                    (id_salt, "exponent"),
                    &mut power_law.exponent,
                    self.value_type,
                    self.min_value,
                    self.max_value,
                    self.speed,
                    current_distr_param_idx,
                    distr_param_names,
                    &mut changed,
                );
            }
        }

        changed
    }

    fn text_to_display(&self, distr_param_names: &[&'static str]) -> String {
        let mut text = String::with_capacity(64);
        write!(&mut text, "{} = ", self.text.label).unwrap();

        match self.distribution.variant {
            DistributionVariant::Constant => {
                self.distribution
                    .constant
                    .append_display_text(&mut text, distr_param_names);
            }
            DistributionVariant::Uniform => {
                self.distribution
                    .uniform()
                    .append_display_text(&mut text, distr_param_names);
            }
            DistributionVariant::UniformCosAngle => {
                self.distribution
                    .uniform_cos_angle()
                    .append_display_text(&mut text, distr_param_names);
            }
            DistributionVariant::PowerLaw => {
                self.distribution
                    .power_law()
                    .append_display_text(&mut text, distr_param_names);
            }
        }
        text
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

    fn uniform(&self) -> &UniformDistribution {
        self.uniform.as_ref().unwrap()
    }

    fn uniform_cos_angle(&self) -> &UniformCosAngleDistribution {
        self.uniform_cos_angle.as_ref().unwrap()
    }

    fn power_law(&self) -> &PowerLawDistribution {
        self.power_law.as_ref().unwrap()
    }

    fn get_or_init_uniform(&mut self) -> &mut UniformDistribution {
        self.uniform
            .get_or_insert_with(|| UniformDistribution::fixed_same(self.constant.fixed))
    }

    fn get_or_init_uniform_cos_angle(&mut self) -> &mut UniformCosAngleDistribution {
        self.uniform_cos_angle
            .get_or_insert_with(|| UniformCosAngleDistribution::fixed_same(self.constant.fixed))
    }

    fn get_or_init_power_law(&mut self) -> &mut PowerLawDistribution {
        self.power_law
            .get_or_insert_with(|| PowerLawDistribution::fixed_same(self.constant.fixed))
    }

    fn to_discrete_param_spec(&self) -> core::DiscreteParamSpec {
        match self.variant {
            DistributionVariant::Constant => {
                core::DiscreteParamSpec::Constant((&self.constant).into())
            }
            DistributionVariant::Uniform => core::DiscreteParamSpec::Uniform {
                min: (&self.uniform().min).into(),
                max: (&self.uniform().max).into(),
            },
            DistributionVariant::UniformCosAngle | DistributionVariant::PowerLaw => unreachable!(),
        }
    }

    fn to_cont_param_spec(&self) -> core::ContParamSpec {
        match self.variant {
            DistributionVariant::Constant => core::ContParamSpec::Constant((&self.constant).into()),
            DistributionVariant::Uniform => core::ContParamSpec::Uniform {
                min: (&self.uniform().min).into(),
                max: (&self.uniform().max).into(),
            },
            DistributionVariant::UniformCosAngle => core::ContParamSpec::UniformCosAngle {
                min_angle: (&self.uniform_cos_angle().min_angle).into(),
                max_angle: (&self.uniform_cos_angle().max_angle).into(),
            },
            DistributionVariant::PowerLaw => core::ContParamSpec::PowerLaw {
                min: (&self.power_law().min).into(),
                max: (&self.power_law().max).into(),
                exponent: (&self.power_law().exponent).into(),
            },
        }
    }
}

impl UniformDistribution {
    fn fixed_same(value: f32) -> Self {
        Self {
            min: ValueSource::fixed(value),
            max: ValueSource::fixed(value),
        }
    }

    fn append_display_text(&self, text: &mut String, distr_param_names: &[&'static str]) {
        text.push_str("uniform(min = ");
        self.min.append_display_text(text, distr_param_names);
        text.push_str(", max = ");
        self.max.append_display_text(text, distr_param_names);
        text.push(')');
    }
}

impl UniformCosAngleDistribution {
    fn fixed_same(value: f32) -> Self {
        Self {
            min_angle: ValueSource::fixed(value),
            max_angle: ValueSource::fixed(value),
        }
    }

    fn append_display_text(&self, text: &mut String, distr_param_names: &[&'static str]) {
        text.push_str("uniformCosAngle(min = ");
        self.min_angle.append_display_text(text, distr_param_names);
        text.push_str(", max = ");
        self.max_angle.append_display_text(text, distr_param_names);
        text.push(')');
    }
}

impl PowerLawDistribution {
    fn fixed_same(value: f32) -> Self {
        Self {
            min: ValueSource::fixed(value),
            max: ValueSource::fixed(value),
            exponent: ValueSource::fixed(1.0),
        }
    }

    fn append_display_text(&self, text: &mut String, distr_param_names: &[&'static str]) {
        text.push_str("powerLaw(min = ");
        self.min.append_display_text(text, distr_param_names);
        text.push_str(", max = ");
        self.max.append_display_text(text, distr_param_names);
        text.push_str(", exponent = ");
        self.exponent.append_display_text(text, distr_param_names);
        text.push(')');
    }
}

const DISCRETE_DISTRIBUTIONS: [DistributionVariant; 2] =
    [DistributionVariant::Constant, DistributionVariant::Uniform];

const CONT_DISTRIBUTIONS: [DistributionVariant; 4] = [
    DistributionVariant::Constant,
    DistributionVariant::Uniform,
    DistributionVariant::UniformCosAngle,
    DistributionVariant::PowerLaw,
];

impl DistributionVariant {
    fn all(value_type: ParamValueType) -> &'static [Self] {
        match value_type {
            ParamValueType::Discrete => &DISCRETE_DISTRIBUTIONS,
            ParamValueType::Continuous => &CONT_DISTRIBUTIONS,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Constant => "Constant",
            Self::Uniform => "Uniform",
            Self::UniformCosAngle => "Uniform cos(angle)",
            Self::PowerLaw => "Power law",
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

    fn append_display_text(&self, text: &mut String, distr_param_names: &[&'static str]) {
        match self.variant {
            ValueSourceVariant::Fixed => {
                write!(text, "{}", self.fixed).unwrap();
            }
            ValueSourceVariant::FromParam => {
                self.from_param.append_display_text(text, distr_param_names);
            }
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

impl FromParamSource {
    fn append_display_text(&self, text: &mut String, distr_param_names: &[&'static str]) {
        self.mapping
            .append_display_text(text, distr_param_names[self.param_idx as usize]);
    }

    fn is_default(&self) -> bool {
        self == &Self::default()
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

impl ParamValueMapping {
    fn append_display_text(&self, text: &mut String, param: &str) {
        match self.variant {
            ParamValueMappingVariant::Linear => {
                self.linear.append_display_text(text, param);
            }
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

impl LinearParamValueMapping {
    fn append_display_text(&self, text: &mut String, param: &str) {
        if self.offset == 0.0 {
            if self.scale == 1.0 {
                write!(text, "'{}'", param).unwrap();
            } else if self.scale == -1.0 {
                write!(text, "-'{}'", param).unwrap();
            } else {
                write!(text, "{} · '{}'", self.scale, param).unwrap();
            }
        } else {
            write!(text, "{}", self.offset).unwrap();
            if self.scale > 0.0 {
                if self.scale == 1.0 {
                    write!(text, " + '{}'", param).unwrap();
                } else {
                    write!(text, " + {} · '{}'", self.scale, param).unwrap();
                }
            } else if self.scale == -1.0 {
                write!(text, " - '{}'", param).unwrap();
            } else {
                write!(text, " - {} · '{}'", -self.scale, param).unwrap();
            }
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

impl ParamValueMappingVariant {
    fn all() -> [Self; 1] {
        [Self::Linear]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
        }
    }
}

fn distribution_combobox(
    ui: &mut Ui,
    value_type: ParamValueType,
    id_salt: impl Hash,
    selected_variant: &mut DistributionVariant,
    changed: &mut bool,
) {
    let variant_before = *selected_variant;
    labeled_option(
        ui,
        LabelAndHoverText {
            label: "Distribution",
            hover_text: "The distribution of parameter values",
        },
        |ui| {
            ComboBox::from_id_salt(id_salt)
                .selected_text(selected_variant.label())
                .show_ui(ui, |ui| {
                    for &variant in DistributionVariant::all(value_type) {
                        ui.selectable_value(selected_variant, variant, variant.label());
                    }
                })
        },
    );
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
    current_distr_param_idx: ParamIdx,
    distr_param_names: &[&'static str],
    changed: &mut bool,
) {
    if distr_param_names.len() > 1 {
        source_combobox(ui, id_salt, &mut source.variant, changed);
    }

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
                (id_salt, "param"),
                &mut from_param.param_idx,
                current_distr_param_idx,
                distr_param_names,
                changed,
            );

            let mapping = &mut from_param.mapping;

            mapping_combobox(ui, (id_salt, "mapping"), &mut mapping.variant, changed);

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
    labeled_option(
        ui,
        LabelAndHoverText {
            label: "Source",
            hover_text: "The source of parameter values",
        },
        |ui| {
            ComboBox::from_id_salt(id_salt)
                .selected_text(selected_variant.label())
                .show_ui(ui, |ui| {
                    for variant in ValueSourceVariant::all() {
                        ui.selectable_value(selected_variant, variant, variant.label());
                    }
                })
        },
    );
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
    if *selected_param_idx == invalid_param_idx {
        *selected_param_idx = (param_names.len() - 1) as ParamIdx;
    }

    let param_idx_before = *selected_param_idx;
    labeled_option(
        ui,
        LabelAndHoverText {
            label: "Parameter",
            hover_text: "The other parameter to depend on",
        },
        |ui| {
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
        },
    );
    if *selected_param_idx != param_idx_before {
        *changed = true;
    }
}

fn mapping_combobox(
    ui: &mut Ui,
    id_salt: impl Hash,
    selected_variant: &mut ParamValueMappingVariant,
    changed: &mut bool,
) {
    let variant_before = *selected_variant;
    labeled_option(
        ui,
        LabelAndHoverText {
            label: "Mapping",
            hover_text: "The mapping from the selected parameter to this parameter",
        },
        |ui| {
            ComboBox::from_id_salt(id_salt)
                .selected_text(selected_variant.label())
                .show_ui(ui, |ui| {
                    for variant in ParamValueMappingVariant::all() {
                        ui.selectable_value(selected_variant, variant, variant.label());
                    }
                })
        },
    );
    if *selected_variant != variant_before {
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
        LabelAndHoverText {
            label: "Value",
            hover_text: "The value of the parameter",
        },
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
        LabelAndHoverText {
            label: "Offset",
            hover_text: "The offset value in the linear mapping",
        },
        DragValue::new(&mut linear.offset).speed(0.05),
    )
    .changed()
    {
        *changed = true;
    }
    if option_drag_value(
        ui,
        LabelAndHoverText {
            label: "Scale",
            hover_text: "The scale factor in the linear mapping",
        },
        DragValue::new(&mut linear.scale).speed(0.05),
    )
    .changed()
    {
        *changed = true;
    }
}
