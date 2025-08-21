//! Generation of shaders from templates.

use crate::shader::ShaderID;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::{Result, anyhow, bail};
use impact_containers::HashSet;
use regex::Regex;
use std::{borrow::Cow, fmt, iter, sync::LazyLock};
use tinyvec::TinyVec;

/// Specific shader template that can be resolved to generate a shader.
pub trait SpecificShaderTemplate: fmt::Debug {
    /// Resolves this instance of the specific shader template into WGSL source
    /// code.
    fn resolve(&self) -> String;

    /// Returns a label describing this instance of the specific shader
    /// template.
    fn label(&self) -> Cow<'static, str> {
        Cow::Owned(format!("{self:?}"))
    }

    /// Returns a unique ID for this template instance (two instances of
    /// specific shader templates should never have the same ID unless they
    /// resolve to the same source code).
    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier(&format!("{self:?}"))
    }
}

/// A shader template that can be resolved to generate a shader.
#[derive(Clone, Debug)]
pub struct ShaderTemplate<'a> {
    source_code: &'a str,
    replacer: Replacer<'a>,
    conditional_blocks: Vec<ConditionalBlock<'a>>,
    flags: Vec<Flag<'a>>,
}

#[derive(Clone, Debug)]
struct Replacer<'a> {
    ac: AhoCorasick,
    patterns: ReplacementPatternSet<'a>,
}

type ReplacementPatternSet<'a> = TinyVec<[&'a str; 8]>;

#[derive(Clone, Debug)]
struct ConditionalBlock<'a> {
    full_text: &'a str,
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

static REPLACEMENT_PATTERN_CAPTURE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\{\{.*?\}\})").unwrap());
static CONDITIONAL_CAPTURE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#if\s*\((.*?)\)\s([\s\S]*?)[^\S\r\n]*(?:#elseif\s*\((.*?)\)\s([\s\S]*?))?[^\S\r\n]*(?:#else\s([\s\S]*?))?[^\S\r\n]*#endif\b").unwrap()
});

impl<'a> ShaderTemplate<'a> {
    /// Creates a new template from the given template source code.
    pub fn new(source_code: &'a str) -> Result<Self> {
        let replacer = Replacer::new(source_code)?;
        let conditional_blocks = find_conditional_blocks(source_code)?;
        let flags = extract_flags(&conditional_blocks);
        Ok(Self {
            source_code,
            replacer,
            conditional_blocks,
            flags,
        })
    }

    pub fn replacement_label_count(&self) -> usize {
        self.replacer.patterns.len()
    }

    pub fn contains_replacement_label(&self, label: &str) -> bool {
        self.replacer
            .patterns
            .iter()
            .any(|pattern| label == label_from_replacement_pattern(pattern))
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
    pub fn resolve<'b>(
        &self,
        flags_to_set: &[&'b str],
        replacements: &[(&'b str, String)],
    ) -> Result<String> {
        for flag in flags_to_set {
            let flag = Flag::new(flag)?;
            if !self.flags.contains(&flag) {
                bail!(
                    "Not all flags to set are present in the template (present flags: {:?})",
                    &self.flags,
                );
            }
        }

        let mut resolved_source_code = Cow::Borrowed(self.source_code);

        for conditional_block in &self.conditional_blocks {
            conditional_block.resolve(flags_to_set, &mut resolved_source_code);
        }

        self.replacer
            .replace(replacements, &mut resolved_source_code)?;

        Ok(resolved_source_code.into_owned())
    }
}

impl<'a> Replacer<'a> {
    fn new(source_code: &'a str) -> Result<Self> {
        let patterns = find_replacement_patterns(source_code)?;

        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&patterns)
            .unwrap();

        Ok(Self { ac, patterns })
    }

    fn replace<'b>(
        &self,
        replacements: &[(&'b str, String)],
        resolved_source_code: &mut Cow<'a, str>,
    ) -> Result<()> {
        if self.patterns.is_empty() {
            return Ok(());
        }

        for (i, (label_a, _)) in replacements.iter().enumerate() {
            for (label_b, _) in &replacements[i + 1..] {
                if label_a == label_b {
                    bail!("Duplicate label `{label_a}` in replacements")
                }
            }
        }

        let source = resolved_source_code.as_ref();
        let mut replaced = String::with_capacity(source.len());
        let mut cursor = 0;

        for m in self.ac.find_iter(source) {
            replaced.push_str(&source[cursor..m.start()]);

            let pattern = self.patterns[m.pattern().as_usize()];
            let label = label_from_replacement_pattern(pattern);

            let replacement = replacements
                .iter()
                .find_map(|(l, rep)| (*l == label).then_some(rep))
                .ok_or_else(|| anyhow!("No label `{label}` to replace in template"))?;

            replaced.push_str(replacement);
            cursor = m.end();
        }

        replaced.push_str(&source[cursor..]);

        *resolved_source_code = Cow::Owned(replaced);

        Ok(())
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
        let if_condition = Condition::new(Flag::new(if_condition)?);
        let elseif_condition = if let Some(elseif_condition) = elseif_condition {
            Some(Condition::new(Flag::new(elseif_condition)?))
        } else {
            None
        };
        Ok(Self {
            full_text,
            if_condition,
            if_body,
            elseif_condition,
            elseif_body,
            else_body,
        })
    }

    fn flags(&self) -> impl Iterator<Item = Flag<'a>> {
        self.if_condition.flags().chain(
            self.elseif_condition
                .as_ref()
                .map(|elseif_condition| elseif_condition.flags())
                .into_iter()
                .flatten(),
        )
    }

    fn resolve<'b>(&self, set_flags: &[&'b str], resolved_source_code: &mut Cow<'a, str>) {
        let replacement_text = if self.if_condition.is_true(set_flags) {
            self.if_body
        } else if self
            .elseif_condition
            .as_ref()
            .is_some_and(|elseif_condition| elseif_condition.is_true(set_flags))
        {
            self.elseif_body.unwrap()
        } else {
            self.else_body.unwrap_or("")
        };

        let source = resolved_source_code.as_ref();

        let mut replaced = String::with_capacity(source.len());
        let mut cursor = 0;
        while let Some(i) = source[cursor..].find(self.full_text) {
            let i = cursor + i;
            replaced.push_str(&source[cursor..i]);
            replaced.push_str(replacement_text);
            cursor = i + self.full_text.len();
        }
        replaced.push_str(&source[cursor..]);

        *resolved_source_code = Cow::Owned(replaced);
    }
}

impl<'a> Condition<'a> {
    fn new(flag: Flag<'a>) -> Self {
        Self { flag }
    }

    fn flags(&self) -> impl Iterator<Item = Flag<'a>> {
        iter::once(self.flag)
    }

    fn is_true(&self, set_flags: &[&'a str]) -> bool {
        set_flags.contains(&self.flag.name)
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
                .map(|(label, replacement)| Cow::Owned(format!("{label} = {replacement}"))),
        )
        .collect::<Vec<_>>()
        .join(", ")
}

fn find_replacement_patterns(source_code: &str) -> Result<ReplacementPatternSet<'_>> {
    let mut patterns = ReplacementPatternSet::new();
    for captures in REPLACEMENT_PATTERN_CAPTURE_REGEX.captures_iter(source_code) {
        if let Some(pattern) = captures.get(1) {
            let pattern = pattern.as_str();
            let label = label_from_replacement_pattern(pattern);
            if !is_valid_identifier(label) {
                bail!(
                    "Invalid label in template (only alphanumeric characters and underscores are allowed): {}",
                    label
                );
            }
            if !patterns.contains(&pattern) {
                patterns.push(pattern);
            }
        }
    }
    Ok(patterns)
}

fn label_from_replacement_pattern(pattern: &str) -> &str {
    &pattern[2..pattern.len() - 2]
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

fn extract_flags<'a>(conditional_blocks: &[ConditionalBlock<'a>]) -> Vec<Flag<'a>> {
    let mut flags = Vec::with_capacity(conditional_blocks.len());
    for conditional_block in conditional_blocks {
        for flag in conditional_block.flags() {
            if !flags.contains(&flag) {
                flags.push(flag);
            }
        }
    }
    flags
}

fn is_valid_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[allow(unreachable_code, unused)]
pub fn validate_template(template: &impl SpecificShaderTemplate) {
    // Skip validation when using `miri` since `wgsl::parse_str` and `regex`
    // is too slow
    #[cfg(miri)]
    return;

    let source = template.resolve();

    println!("{}\n", &source);
    let module = naga::front::wgsl::parse_str(&source).expect("Parsing resolved template failed");
    validate_module(&module);
}

#[allow(clippy::dbg_macro)]
fn validate_module(module: &naga::Module) {
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    if let Err(err) = validator.validate(module) {
        println!("{module:?}");
        eprintln!("{}", err.emit_to_string("test"));
        panic!("Shader validation failed");
    }
}

// `regex` gets very slow under `miri`
#[cfg(not(miri))]
#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(
                ShaderTemplate::new(templ)
                    .unwrap()
                    .obtain_flags()
                    .is_empty()
            );
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
            println!("{templ}");
            assert!(ShaderTemplate::new(templ).is_err());
        }
    }

    #[test]
    fn should_find_no_labels_for_empty_template() {
        let template = ShaderTemplate::new("").unwrap();
        assert_eq!(template.replacement_label_count(), 0);
    }

    #[test]
    fn should_find_correct_label_for_template_with_only_label() {
        let template = ShaderTemplate::new("{{test}}").unwrap();
        assert_eq!(template.replacement_label_count(), 1);
        assert!(template.contains_replacement_label("test"));
    }

    #[test]
    fn should_find_correct_label_for_template_with_only_same_label_twice() {
        let template = ShaderTemplate::new("{{test}}{{test}}").unwrap();
        assert_eq!(template.replacement_label_count(), 1);
        assert!(template.contains_replacement_label("test"));
    }

    #[test]
    fn should_find_correct_labels_for_template_with_only_two_labels() {
        let template = ShaderTemplate::new("{{test1}}{{test2}}").unwrap();
        assert_eq!(template.replacement_label_count(), 2);
        assert!(template.contains_replacement_label("test1"));
        assert!(template.contains_replacement_label("test2"));
    }

    #[test]
    fn should_find_correct_label_for_template_with_label_and_other_stuff() {
        let template = ShaderTemplate::new("{ {{test}}test}_").unwrap();
        assert_eq!(template.replacement_label_count(), 1);
        assert!(template.contains_replacement_label("test"));
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
        assert!(template.resolve(&[], &[]).unwrap().is_empty());
    }

    #[test]
    fn should_fail_to_resolve_empty_template_with_set_flag() {
        let template = ShaderTemplate::new("").unwrap();
        assert!(template.resolve(&["flag"], &[]).is_err());
    }

    #[test]
    fn should_fail_to_resolve_template_with_missing_flag() {
        let template = ShaderTemplate::new("#if (flag) #endif").unwrap();
        assert!(template.resolve(&["otherflag"], &[]).is_err());
    }

    #[test]
    fn should_resolve_template_with_empty_if_block() {
        let template = ShaderTemplate::new("#if (flag) #endif").unwrap();
        assert_eq!(template.resolve(&["flag"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_empty_if_else_block() {
        let template = ShaderTemplate::new("#if (flag) #else #endif").unwrap();
        assert_eq!(template.resolve(&["flag"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_empty_if_elseif_block() {
        let template = ShaderTemplate::new("#if (flag1) #elseif (flag2) #endif").unwrap();
        assert_eq!(template.resolve(&["flag1", "flag2"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&["flag1"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&["flag2"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_empty_if_elseif_else_block() {
        let template = ShaderTemplate::new("#if (flag1) #elseif (flag2) #else #endif").unwrap();
        assert_eq!(template.resolve(&["flag1", "flag2"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&["flag1"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&["flag2"], &[]).unwrap(), "");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_simple_if_block() {
        let template = ShaderTemplate::new("#if (flag) content #endif").unwrap();
        assert_eq!(template.resolve(&["flag"], &[]).unwrap(), "content");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_simple_if_else_block() {
        let template = ShaderTemplate::new("#if (flag) content #else othercontent #endif").unwrap();
        assert_eq!(template.resolve(&["flag"], &[]).unwrap(), "content");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "othercontent");
    }

    #[test]
    fn should_resolve_template_with_simple_if_elseif_block() {
        let template =
            ShaderTemplate::new("#if (flag1) content #elseif (flag2) othercontent #endif").unwrap();
        assert_eq!(
            template.resolve(&["flag1", "flag2"], &[]).unwrap(),
            "content"
        );
        assert_eq!(template.resolve(&["flag1"], &[]).unwrap(), "content");
        assert_eq!(template.resolve(&["flag2"], &[]).unwrap(), "othercontent");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
    }

    #[test]
    fn should_resolve_template_with_simple_if_elseif_else_block() {
        let template = ShaderTemplate::new(
            "#if (flag1) content #elseif (flag2) othercontent #else yetothercontent #endif",
        )
        .unwrap();
        assert_eq!(
            template.resolve(&["flag1", "flag2"], &[]).unwrap(),
            "content"
        );
        assert_eq!(template.resolve(&["flag1"], &[]).unwrap(), "content");
        assert_eq!(template.resolve(&["flag2"], &[]).unwrap(), "othercontent");
        assert_eq!(template.resolve(&[], &[]).unwrap(), "yetothercontent");
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
            template.resolve(&["flag1", "flag2"], &[]).unwrap(),
            "\
            content1\n\
            <other code>\n\
            content2\n\
            "
        );
        assert_eq!(
            template.resolve(&["flag1"], &[]).unwrap(),
            "\
            content1\n\
            <other code>\n\
            othercontent2\n\
            "
        );
        assert_eq!(
            template.resolve(&["flag2"], &[]).unwrap(),
            "\
            othercontent1\n\
            <other code>\n\
            content2\n\
            "
        );
        assert_eq!(
            template.resolve(&[], &[]).unwrap(),
            "\
            othercontent1\n\
            <other code>\n\
            othercontent2\n\
            "
        );
    }

    #[test]
    fn should_fail_to_resolve_with_duplicate_replacement() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let result = template.resolve(
            &[],
            &[
                ("label", "actual1".to_string()),
                ("label", "actual2".to_string()),
            ],
        );
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_replacement_label_missing_from_template() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let result = template.resolve(&[], &[("notlabel", "actual".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_resolve_with_too_few_replacements() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let result = template.resolve(&[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn should_resolve_template_with_only_label() {
        let template = ShaderTemplate::new("{{label}}").unwrap();
        let resolved = template
            .resolve(&[], &[("label", "actual".to_string())])
            .unwrap();
        assert_eq!(&resolved, "actual");
    }

    #[test]
    fn should_resolve_template_with_only_same_label_twice() {
        let template = ShaderTemplate::new("{{label}}{{label}}").unwrap();
        let resolved = template
            .resolve(&[], &[("label", "actual".to_string())])
            .unwrap();
        assert_eq!(&resolved, "actualactual");
    }

    #[test]
    fn should_resolve_template_with_only_two_labels() {
        let template = ShaderTemplate::new("{{label1}}{{label2}}").unwrap();
        let resolved = template
            .resolve(
                &[],
                &[
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
                &[],
                &[
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
        assert_eq!(template.resolve(&[], &[]).unwrap(), "");
        assert!(template.resolve(&["flag"], &[]).is_err());
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
                    &["flag1"],
                    &[
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
