//! Generation of shaders from templates.

pub mod ambient_light;
pub mod ambient_occlusion_application;
pub mod ambient_occlusion_computation;
pub mod bloom_blending;
pub mod bloom_downsampling;
pub mod bloom_upsampling_blur;
pub mod gaussian_blur;
pub mod luminance_histogram;
pub mod luminance_histogram_average;
pub mod model_depth_prepass;
pub mod model_geometry;
pub mod omnidirectional_light;
pub mod omnidirectional_light_shadow_map;
pub mod passthrough;
pub mod render_attachment_visualization;
pub mod skybox;
pub mod temporal_anti_aliasing;
pub mod tone_mapping;
pub mod unidirectional_light;
pub mod unidirectional_light_shadow_map;
pub mod voxel_chunk_culling;
pub mod voxel_geometry;

use crate::gpu::{
    push_constant::PushConstantGroup,
    rendering::{render_command::StencilValue, surface::RenderingSurface},
    resource_group::GPUResourceGroupID,
    shader::ShaderID,
    texture::attachment::{
        RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
    },
};
use anyhow::{bail, Result};
use regex::Regex;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt, iter,
    sync::LazyLock,
};

/// Specific shader template that can be resolved to generate a shader.
pub trait SpecificShaderTemplate: fmt::Debug {
    /// Resolves this instance of the specific shader template into WGSL source
    /// code.
    fn resolve(&self) -> String;

    /// Returns a label describing this instance of the specific shader
    /// template.
    fn label(&self) -> Cow<'static, str> {
        Cow::Owned(format!("{:?}", self))
    }

    /// Returns a unique ID for this template instance (two instances of
    /// specific shader templates should never have the same ID unless they
    /// resolve to the same source code).
    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier(&format!("{:?}", self))
    }
}

/// Specific shader template that can be resolved to generate a compute shader.
pub trait ComputeShaderTemplate: SpecificShaderTemplate + Send + Sync {
    /// Returns the group of push constants used by the compute shader.
    fn push_constants(&self) -> PushConstantGroup;

    /// Returns the set of render attachments used as input by the compute
    /// shader.
    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet;

    /// Returns the ID of the GPU resource group used by the compute shader.
    fn gpu_resource_group_id(&self) -> GPUResourceGroupID;

    /// Returns the workgroup counts to use when invoking the compute shader
    /// based on the dimensions of the rendering surface.
    fn determine_workgroup_counts(&self, rendering_surface: &RenderingSurface) -> [u32; 3];
}

/// Specific shader template that can be resolved to generate a postprocessing
/// shader.
pub trait PostprocessingShaderTemplate: SpecificShaderTemplate {
    /// Returns the group of push constants used by the shader.
    fn push_constants(&self) -> PushConstantGroup;

    /// Returns the set of render attachments used as input by the shader.
    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet;

    /// Returns the descriptions of the render attachments that the shader will
    /// write to.
    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet;

    /// Whether the shader uses the camera projection uniform.
    fn uses_camera(&self) -> bool {
        false
    }

    /// Returns the ID of the GPU resource group used by the shader, or [`None`]
    /// if the shader does not use a GPU resource group.
    fn gpu_resource_group_id(&self) -> Option<GPUResourceGroupID> {
        None
    }

    /// Returns the comparison function and stencil value to use for stencil
    /// testing when using the shader, or [`None`] if stencil testing should not
    /// be used.
    fn stencil_test(&self) -> Option<(wgpu::CompareFunction, StencilValue)> {
        None
    }

    /// Whether the shader writes to the actual surface texture that will be
    /// displayed.
    fn writes_to_surface(&self) -> bool {
        false
    }
}

/// A shader template that can be resolved to generate a shader.
#[derive(Clone, Debug)]
pub struct ShaderTemplate<'a> {
    source_code: &'a str,
    replacement_regexes: HashMap<&'a str, Regex>,
    conditional_blocks: Vec<ConditionalBlock<'a>>,
    flags: HashSet<Flag<'a>>,
}

#[derive(Clone, Debug)]
struct ConditionalBlock<'a> {
    full_text_regex: Regex,
    if_condition: Condition<'a>,
    if_body: &'a str,
    elseif_condition: Option<Condition<'a>>,
    elseif_body: Option<&'a str>,
    else_body: Option<&'a str>,
}

#[derive(Clone, Debug)]
struct Condition<'a> {
    flag: Flag<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Flag<'a> {
    name: &'a str,
}

static REPLACEMENT_LABEL_CAPTURE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{(.*?)\}\}").unwrap());
static CONDITIONAL_CAPTURE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#if\s*\((.*?)\)\s([\s\S]*?)[^\S\r\n]*(?:#elseif\s*\((.*?)\)\s([\s\S]*?))?[^\S\r\n]*(?:#else\s([\s\S]*?))?[^\S\r\n]*#endif\b").unwrap()
});

#[macro_export]
macro_rules! template_source {
    ($type:expr, $name:expr) => {{
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/",
            $type,
            "/",
            $name,
            ".template.wgsl"
        ))
    }};
}

#[macro_export]
macro_rules! rendering_template_source {
    ($name:expr) => {{
        $crate::template_source!("rendering", $name)
    }};
}

#[macro_export]
macro_rules! compute_template_source {
    ($name:expr) => {{
        $crate::template_source!("compute", $name)
    }};
}

#[macro_export]
macro_rules! template_replacements {
    ($($label:literal => $replacement:expr),* $(,)?) => {
        [$(($label, ($replacement).to_string())),*]
    };
}

impl<'a> ShaderTemplate<'a> {
    /// Creates a new template from the given template source code.
    pub fn new(source_code: &'a str) -> Result<Self> {
        let replacement_regexes = find_replacement_labels(source_code)?
            .into_iter()
            .map(|label| {
                (
                    label,
                    Regex::new(&format!("\\{{\\{{{}\\}}\\}}", label)).unwrap(),
                )
            })
            .collect();

        let conditional_blocks = find_conditional_blocks(source_code)?;

        let flags = extract_flags(&conditional_blocks);

        Ok(Self {
            source_code,
            replacement_regexes,
            conditional_blocks,
            flags,
        })
    }

    /// Creates and returns a [`HashSet`] containing the replacement labels in
    /// the template.
    pub fn obtain_replacement_label_set(&self) -> HashSet<&'a str> {
        self.replacement_regexes.keys().copied().collect()
    }

    /// Creates and returns a [`HashSet`] containing the full set of flags used
    /// in the template's conditional blocks.
    pub fn obtain_flags(&self) -> HashSet<&'a str> {
        self.flags.iter().map(|flag| flag.name).collect()
    }

    /// Resolves the template with the given flags set and with the given
    /// replacements. The set flags are used to selectively include or exclude
    /// code in conditional blocks. Each replacement specifies a label in
    /// the template (an identifier surrounded by double curly braces:
    /// `{{<some label>}}`) and the string to replace each occurrence of the
    /// label with.
    ///
    /// # Errors
    /// Returns an error if:
    /// - A flag in `flags_to_set` has invalid syntax (only alphanumeric
    ///   characters and underscores are allowed).
    /// - A flag in `flags_to_set` does not exist in the template.
    /// - A label in `replacements` does not exist in the template after
    ///   resolving all conditional blocks.
    /// - The same label occurs multiple times in `replacements`.
    /// - Not all labels in the template afer resolving conditional blocks are
    ///   included in `replacements`.
    pub fn resolve<'b>(
        &self,
        flags_to_set: impl IntoIterator<Item = &'b str>,
        replacements: impl IntoIterator<Item = (&'b str, String)>,
    ) -> Result<String> {
        let mut resolved_source_code = Cow::Borrowed(self.source_code);

        let mut set_flags = HashSet::new();
        for flag in flags_to_set {
            set_flags.insert(Flag::new(flag)?);
        }

        if !set_flags.is_subset(&self.flags) {
            bail!(
                "Not all flags to set are present in the template (present flags: {:?})",
                &self.flags,
            );
        }

        for conditional_block in &self.conditional_blocks {
            conditional_block.resolve(&set_flags, &mut resolved_source_code);
        }

        let mut replacement_regexes = self.replacement_regexes.clone();
        replacement_regexes.retain(|_, regex| regex.is_match(&resolved_source_code));

        let mut replaced_label_count = 0;
        for (label, replacement) in replacements {
            let replacement_regex = if let Some(replacement_regex) = replacement_regexes.get(label)
            {
                replacement_regex
            } else if self.replacement_regexes.contains_key(label) {
                // The label to replace exists in an excluded conditional block, so we just skip
                // it instead of returning an error
                continue;
            } else {
                bail!("No label `{}` to replace in template", label)
            };

            *resolved_source_code.to_mut() = replacement_regex
                .replace_all(&resolved_source_code, replacement)
                .into_owned();

            replaced_label_count += 1;
        }

        if replaced_label_count < replacement_regexes.len() {
            bail!(
                "Not all labels replaced in template (all labels to replace: {})",
                replacement_regexes
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if replaced_label_count > replacement_regexes.len() {
            bail!("Tried to replace same label multiple times");
        }

        Ok(resolved_source_code.into_owned())
    }
}

impl<'a> ConditionalBlock<'a> {
    fn new(
        full_text: &'a str,
        if_condition: &'a str,
        if_body: &'a str,
        elseif_condition: Option<&'a str>,
        elseif_body: Option<&'a str>,
        else_body: Option<&'a str>,
    ) -> Result<Self> {
        let full_text_regex = Regex::new(&regex::escape(full_text)).unwrap();
        let if_condition = Condition::new(Flag::new(if_condition)?);
        let elseif_condition = if let Some(elseif_condition) = elseif_condition {
            Some(Condition::new(Flag::new(elseif_condition)?))
        } else {
            None
        };
        Ok(Self {
            full_text_regex,
            if_condition,
            if_body,
            elseif_condition,
            elseif_body,
            else_body,
        })
    }

    fn flags(&self) -> impl Iterator<Item = Flag<'a>> + '_ {
        self.if_condition.flags().chain(
            self.elseif_condition
                .as_ref()
                .map(|elseif_condition| elseif_condition.flags())
                .into_iter()
                .flatten(),
        )
    }

    fn resolve<'b>(&self, set_flags: &HashSet<Flag<'b>>, resolved_source_code: &mut Cow<'a, str>) {
        let replacement_text = if self.if_condition.is_true(set_flags) {
            self.if_body
        } else if self
            .elseif_condition
            .as_ref()
            .map_or(false, |elseif_condition| {
                elseif_condition.is_true(set_flags)
            })
        {
            self.elseif_body.unwrap()
        } else {
            self.else_body.unwrap_or("")
        };

        *resolved_source_code.to_mut() = self
            .full_text_regex
            .replace_all(resolved_source_code, replacement_text)
            .into_owned();
    }
}

impl<'a> Condition<'a> {
    fn new(flag: Flag<'a>) -> Self {
        Self { flag }
    }

    fn flags(&self) -> impl Iterator<Item = Flag<'a>> + '_ {
        iter::once(self.flag)
    }

    fn is_true(&self, set_flags: &HashSet<Flag<'a>>) -> bool {
        set_flags.contains(&self.flag)
    }
}

impl<'a> Flag<'a> {
    fn new(name: &'a str) -> Result<Self> {
        if !is_valid_identifier(name) {
            bail!(
                "Invalid flag name (only alphanumeric characters and underscores are allowed): {}",
                name
            );
        }
        Ok(Self { name })
    }
}

/// Creates a unique ID for the shader resolved from a template with the given
/// name using the given flags and replacements.
pub fn create_shader_id_for_template<'b>(
    template_name: &str,
    flags_to_set: impl IntoIterator<Item = &'b str>,
    replacements: impl IntoIterator<Item = (&'b str, String)>,
) -> ShaderID {
    ShaderID::from_identifier(&format!(
        "{}{{ {} }}",
        template_name,
        create_flag_and_replacement_list_string(flags_to_set, replacements)
    ))
}

/// Creates a string listing the given flags and a label and replacement string
/// for each of the given replacements.
fn create_flag_and_replacement_list_string<'b>(
    flags_to_set: impl IntoIterator<Item = &'b str>,
    replacements: impl IntoIterator<Item = (&'b str, String)>,
) -> String {
    flags_to_set
        .into_iter()
        .map(Cow::Borrowed)
        .chain(
            replacements
                .into_iter()
                .map(|(label, replacement)| Cow::Owned(format!("{} = {}", label, replacement))),
        )
        .collect::<Vec<_>>()
        .join(", ")
}

fn find_replacement_labels(source_code: &str) -> Result<HashSet<&str>> {
    let mut labels = HashSet::new();
    for captures in REPLACEMENT_LABEL_CAPTURE_REGEX.captures_iter(source_code) {
        if let Some(label) = captures.get(1) {
            let label = label.as_str();
            if !is_valid_identifier(label) {
                bail!("Invalid label in template (only alphanumeric characters and underscores are allowed): {}", label);
            }
            labels.insert(label);
        }
    }
    Ok(labels)
}

fn find_conditional_blocks(source_code: &str) -> Result<Vec<ConditionalBlock<'_>>> {
    let mut conditional_blocks = Vec::new();
    for captures in CONDITIONAL_CAPTURE_REGEX.captures_iter(source_code) {
        let full_text = captures.get(0).unwrap().as_str();
        let if_condition = captures.get(1).unwrap().as_str();
        let if_body = captures.get(2).map_or("", |m| m.as_str());
        let elseif_condition = captures.get(3).map(|m| m.as_str());
        let elseif_body = captures.get(4).map(|m| m.as_str());
        let else_body = captures.get(5).map(|m| m.as_str());
        conditional_blocks.push(ConditionalBlock::new(
            full_text,
            if_condition,
            if_body,
            elseif_condition,
            elseif_body,
            else_body,
        )?);
    }
    Ok(conditional_blocks)
}

fn extract_flags<'a>(conditional_blocks: &[ConditionalBlock<'a>]) -> HashSet<Flag<'a>> {
    let mut flags = HashSet::with_capacity(conditional_blocks.len());
    for conditional_block in conditional_blocks {
        flags.extend(conditional_block.flags());
    }
    flags
}

fn is_valid_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use naga::{
        front::wgsl,
        valid::{Capabilities, ValidationFlags, Validator},
        Module,
    };

    pub fn validate_template(template: &impl SpecificShaderTemplate) {
        let source = template.resolve();

        // Skip validation when using `miri` since `wgsl::parse_str` is too slow
        #[cfg(miri)]
        return;

        println!("{}\n", &source);
        let module = wgsl::parse_str(&source).expect("Parsing resolved template failed");
        validate_module(&module);
    }

    #[allow(clippy::dbg_macro)]
    fn validate_module(module: &Module) {
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        if let Err(err) = validator.validate(module) {
            dbg!(module);
            eprintln!("{}", err.emit_to_string("test"));
            panic!("Shader validation failed");
        }
    }

    #[test]
    fn should_find_no_flags_for_empty_template() {
        let template = ShaderTemplate::new("").unwrap();
        assert!(template.obtain_flags().is_empty());
    }

    #[test]
    fn should_find_correct_if_flag_for_template_with_only_if() {
        let template = ShaderTemplate::new("#if (flag) #endif").unwrap();
        let flags = template.obtain_flags();
        assert_eq!(flags.len(), 1);
        assert!(flags.contains("flag"));
    }

    #[test]
    fn should_find_correct_if_flag_for_template_with_if_and_other_stuff() {
        let template = ShaderTemplate::new("fi#if (flag) #endif#ifend").unwrap();
        let flags = template.obtain_flags();
        assert_eq!(flags.len(), 1);
        assert!(flags.contains("flag"));
    }

    #[test]
    fn should_find_correct_if_flag_for_template_with_if_and_else() {
        let template = ShaderTemplate::new("#if (flag) #else #endif").unwrap();
        let flags = template.obtain_flags();
        assert_eq!(flags.len(), 1);
        assert!(flags.contains("flag"));
    }

    #[test]
    fn should_find_correct_if_flags_for_template_with_if_and_elseif() {
        let template = ShaderTemplate::new("#if (flag1) #elseif (flag2) #endif").unwrap();
        let flags = template.obtain_flags();
        assert_eq!(flags.len(), 2);
        assert!(flags.contains("flag1"));
        assert!(flags.contains("flag2"));
    }

    #[test]
    fn should_find_correct_if_flags_for_template_with_if_and_elseif_and_else() {
        let template = ShaderTemplate::new("#if (flag1) #elseif (flag2) #else #endif").unwrap();
        let flags = template.obtain_flags();
        assert_eq!(flags.len(), 2);
        assert!(flags.contains("flag1"));
        assert!(flags.contains("flag2"));
    }

    #[test]
    fn should_find_no_flags_for_invalid_conditional_blocks() {
        for templ in [
            "#if (flag)",
            "#if (flag) endif",
            "if (flag) #endif",
            "(flag) #endif",
            "#if #endif",
        ] {
            assert!(ShaderTemplate::new(templ)
                .unwrap()
                .obtain_flags()
                .is_empty());
        }
    }

    #[test]
    fn should_yield_errors_for_invalid_flag_syntax() {
        for templ in [
            "#if (.) #endif",
            "#if (flag?) #endif",
            "#if (fl-ag) #endif",
            "#if (flag) #elseif (flag?) #endif",
            "#if (flag) #elseif (fl-ag) #endif",
            "#if () #else #endif",
            "#if () #elseif () #endif",
            "#if () #elseif () #else #endif",
        ] {
            println!("{}", templ);
            assert!(ShaderTemplate::new(templ).is_err());
        }
    }

    #[test]
    fn should_find_no_labels_for_empty_template() {
        let template = ShaderTemplate::new("").unwrap();
        assert!(template.obtain_replacement_label_set().is_empty());
    }

    #[test]
    fn should_find_correct_label_for_template_with_only_label() {
        let template = ShaderTemplate::new("{{test}}").unwrap();
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains("test"));
    }

    #[test]
    fn should_find_correct_label_for_template_with_only_same_label_twice() {
        let template = ShaderTemplate::new("{{test}}{{test}}").unwrap();
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains("test"));
    }

    #[test]
    fn should_find_correct_labels_for_template_with_only_two_labels() {
        let template = ShaderTemplate::new("{{test1}}{{test2}}").unwrap();
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 2);
        assert!(labels.contains("test1"));
        assert!(labels.contains("test2"));
    }

    #[test]
    fn should_find_correct_label_for_template_with_label_and_other_stuff() {
        let template = ShaderTemplate::new("{ {{test}}test}_").unwrap();
        let labels = template.obtain_replacement_label_set();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains("test"));
    }

    #[test]
    fn should_yield_error_for_invalid_label_syntax() {
        for templ in [
            "{{.label}}",
            "{{test?}}",
            "{{te-st}}",
            "{{test }}",
            "{{test} }}",
        ] {
            assert!(ShaderTemplate::new(templ).is_err());
        }
    }

    #[test]
    fn should_give_empty_string_when_resolving_empty_template() {
        let template = ShaderTemplate::new("").unwrap();
        assert!(template.resolve([], []).unwrap().is_empty());
    }

    #[test]
    fn should_fail_to_resolve_empty_template_with_set_flag() {
        let template = ShaderTemplate::new("").unwrap();
        assert!(template.resolve(["flag"], []).is_err());
    }

    #[test]
    fn should_fail_to_resolve_template_with_missing_flag() {
        let template = ShaderTemplate::new("#if (flag) #endif").unwrap();
        assert!(template.resolve(["otherflag"], []).is_err());
    }

    #[test]
    fn should_resolve_template_with_empty_if_block() {
        let template = ShaderTemplate::new("#if (flag) #endif").unwrap();
        assert_eq!(template.resolve(["flag"], []).unwrap(), "");
        assert_eq!(template.resolve([], []).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_empty_if_else_block() {
        let template = ShaderTemplate::new("#if (flag) #else #endif").unwrap();
        assert_eq!(template.resolve(["flag"], []).unwrap(), "");
        assert_eq!(template.resolve([], []).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_empty_if_elseif_block() {
        let template = ShaderTemplate::new("#if (flag1) #elseif (flag2) #endif").unwrap();
        assert_eq!(template.resolve(["flag1", "flag2"], []).unwrap(), "");
        assert_eq!(template.resolve(["flag1"], []).unwrap(), "");
        assert_eq!(template.resolve(["flag2"], []).unwrap(), "");
        assert_eq!(template.resolve([], []).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_empty_if_elseif_else_block() {
        let template = ShaderTemplate::new("#if (flag1) #elseif (flag2) #else #endif").unwrap();
        assert_eq!(template.resolve(["flag1", "flag2"], []).unwrap(), "");
        assert_eq!(template.resolve(["flag1"], []).unwrap(), "");
        assert_eq!(template.resolve(["flag2"], []).unwrap(), "");
        assert_eq!(template.resolve([], []).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_simple_if_block() {
        let template = ShaderTemplate::new("#if (flag) content #endif").unwrap();
        assert_eq!(template.resolve(["flag"], []).unwrap(), "content");
        assert_eq!(template.resolve([], []).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_simple_if_else_block() {
        let template = ShaderTemplate::new("#if (flag) content #else othercontent #endif").unwrap();
        assert_eq!(template.resolve(["flag"], []).unwrap(), "content");
        assert_eq!(template.resolve([], []).unwrap(), "othercontent");
    }

    #[test]
    fn should_resolve_template_with_simple_if_elseif_block() {
        let template =
            ShaderTemplate::new("#if (flag1) content #elseif (flag2) othercontent #endif").unwrap();
        assert_eq!(template.resolve(["flag1", "flag2"], []).unwrap(), "content");
        assert_eq!(template.resolve(["flag1"], []).unwrap(), "content");
        assert_eq!(template.resolve(["flag2"], []).unwrap(), "othercontent");
        assert_eq!(template.resolve([], []).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_simple_if_elseif_else_block() {
        let template = ShaderTemplate::new(
            "#if (flag1) content #elseif (flag2) othercontent #else yetothercontent #endif",
        )
        .unwrap();
        assert_eq!(template.resolve(["flag1", "flag2"], []).unwrap(), "content");
        assert_eq!(template.resolve(["flag1"], []).unwrap(), "content");
        assert_eq!(template.resolve(["flag2"], []).unwrap(), "othercontent");
        assert_eq!(template.resolve([], []).unwrap(), "yetothercontent");
    }

    #[test]
    fn should_resolve_template_with_multiple_conditional_blocks() {
        let template = ShaderTemplate::new(
            "\
            #if (flag1)\n\
                content1\n\
            #else\n\
                othercontent1\n\
            #endif\
            <other code>\n\
            #if (flag2)\n\
                content2\n\
            #else\n\
                othercontent2\n\
            #endif\
            ",
        )
        .unwrap();
        assert_eq!(
            template.resolve(["flag1", "flag2"], []).unwrap(),
            "\
            content1\n\
            <other code>\n\
            content2\n\
            "
        );
        assert_eq!(
            template.resolve(["flag1"], []).unwrap(),
            "\
            content1\n\
            <other code>\n\
            othercontent2\n\
            "
        );
        assert_eq!(
            template.resolve(["flag2"], []).unwrap(),
            "\
            othercontent1\n\
            <other code>\n\
            content2\n\
            "
        );
        assert_eq!(
            template.resolve([], []).unwrap(),
            "\
            othercontent1\n\
            <other code>\n\
            othercontent2\n\
            "
        );
    }

    #[test]
    fn should_fail_to_resolve_empty_template_with_replacement() {
        let template = ShaderTemplate::new("").unwrap();
        assert!(template
            .resolve([], [("label", "actual".to_string())])
            .is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_duplicate_replacement() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let result = template.resolve(
            [],
            [
                ("label", "actual1".to_string()),
                ("label", "actual2".to_string()),
            ],
        );
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_replacement_label_missing_from_template() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let result = template.resolve([], [("notlabel", "actual".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_too_few_replacements() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let result = template.resolve([], []);
        assert!(result.is_err());
    }

    #[test]
    fn should_resolve_template_with_only_label() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let resolved = template
            .resolve([], [("label", "actual".to_string())])
            .unwrap();
        assert_eq!(&resolved, "actual");
    }

    #[test]
    fn should_resolve_template_with_only_same_label_twice() {
        let template = ShaderTemplate::new("{{label}}{{label}}").unwrap();
        let resolved = template
            .resolve([], [("label", "actual".to_string())])
            .unwrap();
        assert_eq!(&resolved, "actualactual");
    }

    #[test]
    fn should_resolve_template_with_only_two_labels() {
        let template = ShaderTemplate::new("{{label1}}{{label2}}").unwrap();
        let resolved = template
            .resolve(
                [],
                [
                    ("label1", "actual1".to_string()),
                    ("label2", "actual2".to_string()),
                ],
            )
            .unwrap();
        assert_eq!(&resolved, "actual1actual2");
    }

    #[test]
    fn should_resolve_template_with_two_labels_and_other_stuff() {
        let template = ShaderTemplate::new("{ {{label1}}label1{{label2}}_").unwrap();
        let resolved = template
            .resolve(
                [],
                [
                    ("label1", "actual1".to_string()),
                    ("label2", "actual2".to_string()),
                ],
            )
            .unwrap();
        assert_eq!(&resolved, "{ actual1label1actual2_");
    }

    #[test]
    fn should_only_require_label_in_taken_conditional_branch() {
        let template = ShaderTemplate::new("#if (flag) {{label}} #endif").unwrap();
        assert_eq!(template.resolve([], []).unwrap(), "");
        assert!(template.resolve(["flag"], []).is_err());
    }

    #[test]
    fn should_resolve_template_with_multiple_labels_and_conditional_blocks() {
        let template = ShaderTemplate::new(
            "\
            {{label1}}\n\
            #if (flag1)\n\
                {{label2}}\n\
            #else\n\
                othercontent1\n\
            #endif\
            <other code>\n\
            #if (flag2)\n\
                content2\n\
            #else\n\
                {{label3}}\n\
            #endif\
            ",
        )
        .unwrap();
        assert_eq!(
            template
                .resolve(
                    ["flag1"],
                    [
                        ("label1", "actual1".to_string()),
                        ("label2", "actual2".to_string()),
                        ("label3", "actual3".to_string()),
                    ]
                )
                .unwrap(),
            "\
            actual1\n\
            actual2\n\
            <other code>\n\
            actual3\n\
            "
        );
    }
}
