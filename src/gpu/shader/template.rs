//! Generation of shaders from templates.

use crate::gpu::{
    shader::{Shader, ShaderID},
    GraphicsDevice,
};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

/// Specific shader templates that can be resolved to generate shaders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecificShaderTemplate {
    Passthrough,
    AmbientOcclusionComputation,
    AmbientOcclusionApplication,
    GaussianBlur,
    LuminanceHistogram,
    LuminanceHistogramAverage,
    ToneMapping,
}

/// A shader template that can be resolved to generate a shader.
#[derive(Clone, Debug)]
pub struct ShaderTemplate<'a> {
    source_code: &'a str,
    replacement_regexes: HashMap<&'a str, Regex>,
}

lazy_static! {
    static ref REPLACEMENT_LABEL_CAPTURE_REGEX: Regex = Regex::new(r"\{\{(\w+)\}\}").unwrap();
    static ref PASSTHROUGH_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::Passthrough.wgsl_source());
    static ref AMBIENT_OCCLUSION_COMPUTATION_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::AmbientOcclusionComputation.wgsl_source());
    static ref AMBIENT_OCCLUSION_APPLICATION_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::AmbientOcclusionApplication.wgsl_source());
    static ref GAUSSIAN_BLUR_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::GaussianBlur.wgsl_source());
    static ref LUMINANCE_HISTOGRAM_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::LuminanceHistogram.wgsl_source());
    static ref LUMINANCE_HISTOGRAM_AVERAGE_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::LuminanceHistogramAverage.wgsl_source());
    static ref TONE_MAPPING_TEMPLATE: ShaderTemplate<'static> =
        ShaderTemplate::new(SpecificShaderTemplate::ToneMapping.wgsl_source());
}

macro_rules! template_source {
    ($type:expr, $name:expr) => {{
        include_str!(concat!(
            "../../../shaders/",
            $type,
            "/",
            $name,
            ".template.wgsl"
        ))
    }};
}

macro_rules! rendering_template_source {
    ($name:expr) => {{
        template_source!("rendering", $name)
    }};
}

macro_rules! compute_template_source {
    ($name:expr) => {{
        template_source!("compute", $name)
    }};
}

impl SpecificShaderTemplate {
    /// Returns the WGSL source code of the template.
    pub const fn wgsl_source(&self) -> &'static str {
        match self {
            Self::Passthrough => {
                rendering_template_source!("passthrough")
            }
            Self::AmbientOcclusionComputation => {
                rendering_template_source!("ambient_occlusion_computation")
            }
            Self::AmbientOcclusionApplication => {
                rendering_template_source!("ambient_occlusion_application")
            }
            Self::GaussianBlur => {
                rendering_template_source!("gaussian_blur")
            }
            Self::LuminanceHistogram => {
                compute_template_source!("luminance_histogram")
            }
            Self::LuminanceHistogramAverage => {
                compute_template_source!("luminance_histogram_average")
            }
            Self::ToneMapping => {
                rendering_template_source!("tone_mapping")
            }
        }
    }

    /// Returns the [`ShaderTemplate`] for this specific shader template.
    pub fn template(&self) -> &'static ShaderTemplate<'static> {
        match self {
            Self::Passthrough => &PASSTHROUGH_TEMPLATE,
            Self::AmbientOcclusionComputation => &AMBIENT_OCCLUSION_COMPUTATION_TEMPLATE,
            Self::AmbientOcclusionApplication => &AMBIENT_OCCLUSION_APPLICATION_TEMPLATE,
            Self::GaussianBlur => &GAUSSIAN_BLUR_TEMPLATE,
            Self::LuminanceHistogram => &LUMINANCE_HISTOGRAM_TEMPLATE,
            Self::LuminanceHistogramAverage => &LUMINANCE_HISTOGRAM_AVERAGE_TEMPLATE,
            Self::ToneMapping => &TONE_MAPPING_TEMPLATE,
        }
    }
}

impl std::fmt::Display for SpecificShaderTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> ShaderTemplate<'a> {
    /// Creates a new template from the given template source code.
    pub fn new(source_code: &'a str) -> Self {
        let replacement_regexes = find_replacement_labels(source_code)
            .into_iter()
            .map(|label| {
                (
                    label,
                    Regex::new(&format!("\\{{\\{{{}\\}}\\}}", label)).unwrap(),
                )
            })
            .collect();
        Self {
            source_code,
            replacement_regexes,
        }
    }

    /// Creates and returns a [`HashSet`] containing the replacement labels in
    /// the template.
    pub fn obtain_replacement_label_set(&self) -> HashSet<&'a str> {
        self.replacement_regexes.keys().copied().collect()
    }

    /// Resolves the template with the given replacements. Each replacement
    /// specifies a label in the template (an identifier surrounded by double
    /// curly braces: `{{<some label>}}`) and the string to replace each
    /// occurrence of the label with.
    ///
    /// # Errors
    /// Returns an error if:
    /// - A label in `replacements` does not exist in the template.
    /// - The same label occurs multiple times in `replacements`.
    /// - Not all labels in the template are included in `replacements`.
    pub fn resolve<'b>(
        &self,
        replacements: impl IntoIterator<Item = (&'b str, String)>,
    ) -> Result<String> {
        let mut resolved_source_code = Cow::Borrowed(self.source_code);
        let mut replaced_label_count = 0;

        for (label, replacement) in replacements {
            let replacement_regex = self
                .replacement_regexes
                .get(label)
                .ok_or_else(|| anyhow!("No label `{}` to replace in template", label))?;

            resolved_source_code = Cow::Owned(
                replacement_regex
                    .replace_all(&resolved_source_code, replacement)
                    .into_owned(),
            );

            replaced_label_count += 1;
        }

        if replaced_label_count < self.replacement_regexes.len() {
            return Err(anyhow!(
                "Not all labels replaced in template (all labels to replace: {})",
                self.replacement_regexes
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if replaced_label_count > self.replacement_regexes.len() {
            return Err(anyhow!("Tried to replace same label multiple times"));
        }

        Ok(resolved_source_code.into_owned())
    }

    /// Resolves the template (see [`Self::resolve`]) and builds a [`Shader`]
    /// from the resolved source code, assuming the code is WGSL.
    ///
    /// # Errors
    /// See [`Self::resolve`] and [`Shader::from_wgsl_source`].
    pub fn resolve_and_compile_as_wgsl<'b>(
        &self,
        graphics_device: &GraphicsDevice,
        replacements: impl IntoIterator<Item = (&'b str, String)>,
        label: &str,
    ) -> Result<Shader> {
        let resolved_source_code = self.resolve(replacements)?;
        Shader::from_wgsl_source(graphics_device, resolved_source_code, label)
    }
}

/// Creates a unique ID for the shader resolved from a template with the given
/// name using the given replacements.
pub fn create_shader_id_for_template<'b>(
    template_name: &str,
    replacements: impl IntoIterator<Item = (&'b str, String)>,
) -> ShaderID {
    ShaderID::from_identifier(&format!(
        "{}{{ {} }}",
        template_name,
        create_replacement_list_string(replacements)
    ))
}

/// Creates a string listing the label and replacement string for each of the
/// given replacements.
fn create_replacement_list_string<'b>(
    replacements: impl IntoIterator<Item = (&'b str, String)>,
) -> String {
    replacements
        .into_iter()
        .map(|(label, replacement)| format!("{} = {}", label, replacement))
        .collect::<Vec<_>>()
        .join(", ")
}

fn find_replacement_labels(source_code: &str) -> HashSet<&str> {
    let mut labels = HashSet::new();
    for captures in REPLACEMENT_LABEL_CAPTURE_REGEX.captures_iter(source_code) {
        if let Some(label) = captures.get(1) {
            labels.insert(label.as_str());
        }
    }
    labels
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_find_no_labels_for_empty_template() {
        let template = ShaderTemplate::new("");
        assert!(template.obtain_replacement_label_set().is_empty());
    }

    #[test]
    fn should_find_correct_label_for_template_with_only_label() {
        let template = ShaderTemplate::new("{{test}}");
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains("test"));
    }

    #[test]
    fn should_find_correct_label_for_template_with_only_same_label_twice() {
        let template = ShaderTemplate::new("{{test}}{{test}}");
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains("test"));
    }

    #[test]
    fn should_find_correct_labels_for_template_with_only_two_labels() {
        let template = ShaderTemplate::new("{{test1}}{{test2}}");
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains("test1"));
        assert!(labels.contains("test2"));
    }

    #[test]
    fn should_find_correct_label_for_template_with_label_and_other_stuff() {
        let template = ShaderTemplate::new("{{{test}}test}_");
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains("test"));
    }

    #[test]
    fn should_give_empty_string_when_resolving_empty_template() {
        let template = ShaderTemplate::new("");
        assert!(template.resolve([]).unwrap().is_empty());
    }

    #[test]
    fn should_fail_to_resolve_empty_template_with_replacement() {
        let template = ShaderTemplate::new("");
        assert!(template.resolve([("label", "actual".to_string())]).is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_duplacate_replacement() {
        let template = ShaderTemplate::new("{{label}}");
        let result = template.resolve([
            ("label", "actual1".to_string()),
            ("label", "actual2".to_string()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_replacement_label_missing_from_template() {
        let template = ShaderTemplate::new("{{label}}");
        let result = template.resolve([("notlabel", "actual".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_to_few_replacements() {
        let template = ShaderTemplate::new("{{label}}");
        let result = template.resolve([]);
        assert!(result.is_err());
    }

    #[test]
    fn should_resolve_template_with_only_label() {
        let template = ShaderTemplate::new("{{label}}");
        let resolved = template.resolve([("label", "actual".to_string())]).unwrap();
        assert_eq!(&resolved, "actual");
    }

    #[test]
    fn should_resolve_template_with_only_same_label_twice() {
        let template = ShaderTemplate::new("{{label}}{{label}}");
        let resolved = template.resolve([("label", "actual".to_string())]).unwrap();
        assert_eq!(&resolved, "actualactual");
    }

    #[test]
    fn should_resolve_template_with_only_two_labels() {
        let template = ShaderTemplate::new("{{label1}}{{label2}}");
        let resolved = template
            .resolve([
                ("label1", "actual1".to_string()),
                ("label2", "actual2".to_string()),
            ])
            .unwrap();
        assert_eq!(&resolved, "actual1actual2");
    }

    #[test]
    fn should_resolve_template_with_two_labels_and_other_stuff() {
        let template = ShaderTemplate::new("{{{label1}}label1{{label2}}_");
        let resolved = template
            .resolve([
                ("label1", "actual1".to_string()),
                ("label2", "actual2".to_string()),
            ])
            .unwrap();
        assert_eq!(&resolved, "{actual1label1actual2_");
    }
}
