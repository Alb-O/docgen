// Copyright (C) 2024 The nixdoc contributors
//
// nixdoc is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! This module implements rendering of NixOS-style module options to CommonMark.
//!
//! It parses the JSON format produced by `lib.optionAttrSetToDocList` from nixpkgs
//! and renders it to CommonMark documentation.
//!
//! # JSON Format
//!
//! The expected input format is a JSON object where keys are option names and values
//! contain option metadata:
//!
//! ```json
//! {
//!   "services.nginx.enable": {
//!     "loc": ["services", "nginx", "enable"],
//!     "description": "Whether to enable nginx.",
//!     "type": "boolean",
//!     "default": { "_type": "literalExpression", "text": "false" },
//!     "example": { "_type": "literalExpression", "text": "true" },
//!     "declarations": ["nixos/modules/services/web-servers/nginx/default.nix"],
//!     "readOnly": false
//!   }
//! }
//! ```

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A value that can be either a literal expression, literal markdown, or a raw value.
/// This matches the `_type` tagged format used by nixpkgs.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OptionValue {
    /// A literal Nix expression: `{ _type = "literalExpression"; text = "..."; }`
    Tagged(TaggedValue),
    /// A raw string value
    String(String),
    /// A raw boolean value
    Bool(bool),
    /// A raw numeric value
    Number(serde_json::Number),
    /// A raw array value
    Array(Vec<serde_json::Value>),
    /// A raw object value (that doesn't have _type).
    /// Required for deserializing arbitrary Nix attrsets that appear in option values.
    Object(serde_json::Map<String, serde_json::Value>),
    /// Null value
    Null,
}

/// A tagged value with `_type` field
#[derive(Debug, Clone, Deserialize)]
pub struct TaggedValue {
    #[serde(rename = "_type")]
    pub value_type: String,
    pub text: Option<String>,
}

/// Description can be either a plain string or an mdDoc object
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Description {
    /// Plain string description
    Plain(String),
    /// mdDoc tagged description: `{ _type = "mdDoc"; text = "..."; }`
    MdDoc { _type: String, text: String },
}

impl Description {
    pub fn as_str(&self) -> &str {
        match self {
            Description::Plain(s) => s,
            Description::MdDoc { text, .. } => text,
        }
    }
}

/// Represents a single option's metadata as parsed from JSON
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionDef {
    /// The option's location as a list of path segments.
    /// Present in the JSON but not currently used in rendering (we use the key instead).
    /// Could be used for hierarchical navigation in the future.
    #[serde(default)]
    pub loc: Vec<String>,

    /// Human-readable description of the option
    #[serde(default)]
    pub description: Option<Description>,

    /// Type description string (e.g., "boolean", "list of string")
    #[serde(rename = "type", default)]
    pub option_type: Option<String>,

    /// Default value (may be a literalExpression)
    #[serde(default)]
    pub default: Option<OptionValue>,

    /// Example value (may be a literalExpression)
    #[serde(default)]
    pub example: Option<OptionValue>,

    /// Source file declarations
    #[serde(default)]
    pub declarations: Vec<DeclarationLoc>,

    /// Whether the option is read-only
    #[serde(default)]
    pub read_only: bool,

    /// Related packages markdown (pre-rendered)
    #[serde(default)]
    pub related_packages: Option<String>,
}

/// Declaration location can be a string or an object with name and url
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DeclarationLoc {
    /// Simple path string
    Path(String),
    /// Object with name and optional url
    Named { name: String, url: Option<String> },
}

impl DeclarationLoc {
    pub fn name(&self) -> &str {
        match self {
            DeclarationLoc::Path(p) => p,
            DeclarationLoc::Named { name, .. } => name,
        }
    }

    pub fn url(&self) -> Option<&str> {
        match self {
            DeclarationLoc::Path(_) => None,
            DeclarationLoc::Named { url, .. } => url.as_deref(),
        }
    }
}

/// Parsed options from JSON
pub type OptionsMap = HashMap<String, OptionDef>;

/// Parse options JSON from a file
pub fn parse_options_file(path: &Path) -> Result<OptionsMap, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read options file: {}", e))?;
    parse_options_json(&content)
}

/// Parse options JSON from a string
pub fn parse_options_json(json: &str) -> Result<OptionsMap, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to parse options JSON: {}", e))
}

/// Escape special CommonMark characters
fn md_escape(text: &str) -> String {
    // Escape characters that have special meaning in CommonMark
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('`', "\\`")
}

/// Format an option value for display
fn format_option_value(value: &OptionValue) -> String {
    match value {
        OptionValue::Tagged(tagged) => {
            match tagged.value_type.as_str() {
                "literalExpression" => {
                    if let Some(text) = &tagged.text {
                        // Multi-line expressions get code blocks
                        if text.contains('\n') {
                            format!("```nix\n{}\n```", text)
                        } else {
                            format!("`{}`", text)
                        }
                    } else {
                        "`...`".to_string()
                    }
                }
                "literalMD" => {
                    // Literal markdown is rendered as-is
                    tagged.text.clone().unwrap_or_default()
                }
                _ => {
                    // Unknown tagged type
                    format!(
                        "`<{}>: {}`",
                        tagged.value_type,
                        tagged.text.as_deref().unwrap_or("...")
                    )
                }
            }
        }
        OptionValue::String(s) => format!("`\"{}\"`", s),
        OptionValue::Bool(b) => format!("`{}`", b),
        OptionValue::Number(n) => format!("`{}`", n),
        OptionValue::Array(arr) => {
            // Simple array representation
            let items: Vec<String> = arr
                .iter()
                .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "...".to_string()))
                .collect();
            if items.is_empty() {
                "`[ ]`".to_string()
            } else {
                format!("`[ {} ]`", items.join(" "))
            }
        }
        OptionValue::Object(_) => "`{ ... }`".to_string(),
        OptionValue::Null => "`null`".to_string(),
    }
}

/// Create a sanitized anchor ID from an option name
fn make_anchor_id(name: &str, prefix: &str) -> String {
    let sanitized = name
        .replace('.', "-")
        .replace('<', "_")
        .replace('>', "_")
        .replace('*', "_");
    format!("{}{}", prefix, sanitized)
}

/// Options for rendering
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Prefix for anchor IDs (e.g., "opt-")
    pub anchor_prefix: String,
    /// Whether to include declaration links
    pub include_declarations: bool,
    /// Base URL for declaration links (if declarations are relative paths)
    pub declarations_base_url: Option<String>,
    /// Revision for GitHub links
    pub revision: Option<String>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            anchor_prefix: "opt-".to_string(),
            include_declarations: true,
            declarations_base_url: None,
            revision: None,
        }
    }
}

/// Render a single option to CommonMark
fn render_option(name: &str, opt: &OptionDef, opts: &RenderOptions) -> String {
    let mut output = String::new();

    // Header with anchor
    let anchor = make_anchor_id(name, &opts.anchor_prefix);
    output.push_str(&format!("## `{}` {{#{}}}\n\n", name, anchor));

    // Type and read-only status
    if let Some(ref opt_type) = opt.option_type {
        let ro = if opt.read_only { " *(read only)*" } else { "" };
        output.push_str(&format!("**Type:** `{}`{}\n\n", opt_type, ro));
    }

    // Default value
    if let Some(ref default) = opt.default {
        let formatted = format_option_value(default);
        if formatted.contains('\n') {
            output.push_str(&format!("**Default:**\n\n{}\n\n", formatted));
        } else {
            output.push_str(&format!("**Default:** {}\n\n", formatted));
        }
    }

    // Description
    if let Some(ref desc) = opt.description {
        let desc_text = desc.as_str();
        if !desc_text.is_empty() {
            output.push_str(desc_text);
            output.push_str("\n\n");
        }
    }

    // Example
    if let Some(ref example) = opt.example {
        let formatted = format_option_value(example);
        if formatted.contains('\n') {
            output.push_str(&format!("**Example:**\n\n{}\n\n", formatted));
        } else {
            output.push_str(&format!("**Example:** {}\n\n", formatted));
        }
    }

    // Related packages
    if let Some(ref related) = opt.related_packages {
        if !related.is_empty() {
            output.push_str("**Related packages:**\n\n");
            output.push_str(related);
            output.push_str("\n\n");
        }
    }

    // Declarations
    if opts.include_declarations && !opt.declarations.is_empty() {
        output.push_str("**Declared by:**\n\n");
        for decl in &opt.declarations {
            let name = decl.name();
            if let Some(url) = decl.url() {
                output.push_str(&format!("- [{}]({})\n", md_escape(name), url));
            } else if let Some(ref base_url) = opts.declarations_base_url {
                // Build URL from base + path
                let url = if let Some(ref rev) = opts.revision {
                    format!("{}/blob/{}/{}", base_url.trim_end_matches('/'), rev, name)
                } else {
                    format!("{}/blob/master/{}", base_url.trim_end_matches('/'), name)
                };
                output.push_str(&format!("- [{}]({})\n", md_escape(name), url));
            } else {
                output.push_str(&format!("- `{}`\n", name));
            }
        }
        output.push('\n');
    }

    output
}

/// Get sort priority for an option name segment.
/// Returns 0 for "enable*", 1 for "package*", 2 for everything else.
fn segment_priority(segment: &str) -> u8 {
    if segment.starts_with("enable") {
        0
    } else if segment.starts_with("package") {
        1
    } else {
        2
    }
}

/// Compare two option names for sorting.
/// Sorts with enable first, then package, then alphabetically within each segment.
fn compare_option_names(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (a_seg, b_seg) in a_parts.iter().zip(b_parts.iter()) {
        // First compare by priority (enable < package < other)
        let priority_cmp = segment_priority(a_seg).cmp(&segment_priority(b_seg));
        if priority_cmp != std::cmp::Ordering::Equal {
            return priority_cmp;
        }
        // Then alphabetically within same priority
        let alpha_cmp = a_seg.cmp(b_seg);
        if alpha_cmp != std::cmp::Ordering::Equal {
            return alpha_cmp;
        }
    }
    // Shorter paths come first
    a_parts.len().cmp(&b_parts.len())
}

/// Render all options to CommonMark
pub fn render_options_to_commonmark(options: &OptionsMap, render_opts: &RenderOptions) -> String {
    let mut output = String::new();

    // Sort options by name for consistent output
    let mut names: Vec<&String> = options.keys().collect();
    names.sort_by(|a, b| compare_option_names(a, b));

    for name in names {
        if let Some(opt) = options.get(name) {
            output.push_str(&render_option(name, opt, render_opts));
        }
    }

    output
}

/// Render options with a title and optional preamble
pub fn render_options_document(
    options: &OptionsMap,
    title: &str,
    preamble: Option<&str>,
    render_opts: &RenderOptions,
) -> String {
    let mut output = String::new();

    // Title
    output.push_str(&format!("# {}\n\n", title));

    // Preamble
    if let Some(pre) = preamble {
        output.push_str(pre);
        output.push_str("\n\n");
    }

    // Options
    output.push_str(&render_options_to_commonmark(options, render_opts));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_option() {
        let json = r#"{
            "test.enable": {
                "loc": ["test", "enable"],
                "description": "Whether to enable test.",
                "type": "boolean",
                "default": { "_type": "literalExpression", "text": "false" },
                "readOnly": false
            }
        }"#;

        let options = parse_options_json(json).unwrap();
        assert_eq!(options.len(), 1);

        let opt = options.get("test.enable").unwrap();
        assert_eq!(opt.option_type.as_deref(), Some("boolean"));
        assert!(!opt.read_only);
    }

    #[test]
    fn test_render_option() {
        let json = r#"{
            "test.enable": {
                "loc": ["test", "enable"],
                "description": "Whether to enable test.",
                "type": "boolean",
                "default": { "_type": "literalExpression", "text": "false" },
                "example": { "_type": "literalExpression", "text": "true" },
                "readOnly": false
            }
        }"#;

        let options = parse_options_json(json).unwrap();
        let output = render_options_to_commonmark(&options, &RenderOptions::default());

        assert!(output.contains("## `test.enable`"));
        assert!(output.contains("**Type:** `boolean`"));
        assert!(output.contains("**Default:** `false`"));
        assert!(output.contains("Whether to enable test."));
        assert!(output.contains("**Example:** `true`"));
    }
}
