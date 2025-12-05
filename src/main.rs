// Copyright (C) 2018 Vincent Ambo <mail@tazj.in>
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

//! This tool generates CommonMark from a Nix file defining library
//! functions, such as the files in `lib/` in the nixpkgs repository.
//!
//! TODO:
//! * extract function argument names
//! * extract line number & add it to generated output
//! * figure out how to specify examples (& leading whitespace?!)

mod comment;
mod commonmark;
mod format;
mod legacy;
mod options;
#[cfg(test)]
mod test;

use crate::{format::handle_indentation, legacy::retrieve_legacy_comment};

use self::comment::get_expr_docs;
use self::commonmark::*;
use format::shift_headings;
use legacy::{collect_lambda_args_legacy, LegacyDocItem};
use rnix::{
    ast::{Attr, AttrpathValue, Expr, HasEntry, Ident, Inherit, LetIn},
    SyntaxKind, SyntaxNode,
};
use rowan::{ast::AstNode, WalkEvent};
use std::fs;

use serde::Serialize;
use std::collections::HashMap;

use clap::Parser;
use std::path::PathBuf;

/// Command line arguments for nixdoc
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    // Legacy mode arguments (for backwards compatibility)
    /// Prefix for the category (e.g. 'lib' or 'utils').
    #[arg(short, long, default_value_t = String::from("lib"))]
    prefix: String,

    #[arg(long, default_value_t = String::from("function-library-"))]
    anchor_prefix: String,

    /// Whether to output JSON.
    #[arg(short, long, default_value_t = false)]
    json_output: bool,

    /// Name of the function category (e.g. 'strings', 'attrsets').
    #[arg(short, long)]
    category: Option<String>,

    /// Description of the function category.
    #[arg(short, long)]
    description: Option<String>,

    /// Nix file to process.
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Path to a file containing location data as JSON.
    #[arg(short, long)]
    locs: Option<PathBuf>,

    /// Comma-separated list of bindings to export (documents only these from let block).
    /// When specified, ignores what the file returns and documents only these bindings.
    #[arg(short, long, value_delimiter = ',')]
    export: Option<Vec<String>>,
}

#[derive(Debug, Parser)]
enum Command {
    /// Render NixOS-style module options from JSON to CommonMark
    Options {
        /// Input JSON file containing options (from lib.optionAttrSetToDocList)
        #[arg(short, long)]
        file: PathBuf,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Document title
        #[arg(short, long, default_value = "Module Options")]
        title: String,

        /// Preamble text to include after the title
        #[arg(short, long)]
        preamble: Option<String>,

        /// Prefix for anchor IDs
        #[arg(long, default_value = "opt-")]
        anchor_prefix: String,

        /// Include declaration source links
        #[arg(long, default_value_t = true)]
        include_declarations: bool,

        /// Base URL for declaration links (e.g., https://github.com/owner/repo)
        #[arg(long)]
        declarations_base_url: Option<String>,

        /// Git revision for declaration links
        #[arg(long)]
        revision: Option<String>,
    },

    /// Extract just the file-level documentation comment from a Nix file
    FileDoc {
        /// Nix file to extract documentation from
        #[arg(short, long)]
        file: PathBuf,

        /// Output format: markdown, json, or plain
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Shift heading levels by this amount (e.g., 2 turns # into ###)
        #[arg(long, default_value_t = 0)]
        shift_headings: usize,
    },
}

/// Legacy options struct for backwards compatibility
struct LegacyOptions {
    prefix: String,
    anchor_prefix: String,
    json_output: bool,
    category: String,
    description: String,
    file: PathBuf,
    locs: Option<PathBuf>,
    export: Option<Vec<String>>,
}

#[derive(Debug)]
struct DocComment {
    /// Primary documentation string.
    doc: String,

    /// Optional type annotation for the thing being documented.
    /// This is only available as legacy feature
    doc_type: Option<String>,

    /// Usage example(s) (interpreted as a single code block)
    /// This is only available as legacy feature
    example: Option<String>,
}

#[derive(Debug)]
struct DocItem {
    name: String,
    comment: DocComment,
}

#[derive(Debug, Serialize)]
struct JsonFormat {
    version: u32,
    entries: Vec<ManualEntry>,
}

enum DocItemOrLegacy {
    LegacyDocItem(LegacyDocItem),
    DocItem(DocItem),
}

/// Returns a rfc145 doc-comment if one is present
pub fn retrieve_doc_comment(node: &SyntaxNode, shift_headings_by: Option<usize>) -> Option<String> {
    let doc_comment = get_expr_docs(node);

    doc_comment.map(|doc_comment| {
        shift_headings(
            &handle_indentation(&doc_comment).unwrap_or(String::new()),
            // H1 to H4 can be used in the doc-comment with the current rendering.
            // They will be shifted to H3, H6
            // H1 and H2 are currently used by the outer rendering. (category and function name)
            shift_headings_by.unwrap_or(2),
        )
    })
}

/// Transforms an AST node into a `DocItem` if it has a leading
/// documentation comment.
fn retrieve_doc_item(node: &AttrpathValue) -> Option<DocItemOrLegacy> {
    let ident = node.attrpath().unwrap();
    // TODO this should join attrs() with '.' to handle whitespace, dynamic attrs and string
    // attrs. none of these happen in nixpkgs lib, and the latter two should probably be
    // rejected entirely.
    let item_name = ident.to_string();

    let doc_comment = retrieve_doc_comment(node.syntax(), Some(2));
    match doc_comment {
        Some(comment) => Some(DocItemOrLegacy::DocItem(DocItem {
            name: item_name,
            comment: DocComment {
                doc: comment,
                doc_type: None,
                example: None,
            },
        })),
        // Fallback to legacy comment is there is no doc_comment
        None => {
            let comment = retrieve_legacy_comment(node.syntax(), false)?;
            Some(DocItemOrLegacy::LegacyDocItem(LegacyDocItem {
                name: item_name,
                comment: parse_doc_comment(&comment),
                args: vec![],
            }))
        }
    }
}

/// Dumb, mutable, hacky doc comment "parser".
fn parse_doc_comment(raw: &str) -> DocComment {
    enum ParseState {
        Doc,
        Type,
        Example,
    }

    let mut state = ParseState::Doc;

    // Split the string into three parts, docs, type and example
    let mut doc_str = String::new();
    let mut type_str = String::new();
    let mut example_str = String::new();

    for line in raw.split_inclusive('\n') {
        let trimmed_line = line.trim();
        if let Some(suffix) = trimmed_line.strip_prefix("Type:") {
            state = ParseState::Type;
            type_str.push_str(suffix);
            type_str.push('\n');
        } else if let Some(suffix) = trimmed_line.strip_prefix("Example:") {
            state = ParseState::Example;
            example_str.push_str(suffix);
            example_str.push('\n');
        } else {
            match state {
                ParseState::Doc => doc_str.push_str(line),
                ParseState::Type => type_str.push_str(line),
                ParseState::Example => example_str.push_str(line),
            }
        }
    }

    DocComment {
        doc: handle_indentation(&doc_str).unwrap_or(String::new()),
        doc_type: handle_indentation(&type_str),
        example: handle_indentation(&example_str),
    }
}

/// Traverse the arena from a top-level SetEntry and collect, where
/// possible:
///
/// 1. The identifier of the set entry itself.
/// 2. The attached doc comment on the entry.
/// 3. The argument names of any curried functions (pattern functions
///    not yet supported).
fn collect_entry_information(entry: AttrpathValue) -> Option<LegacyDocItem> {
    let doc_item = retrieve_doc_item(&entry)?;

    match doc_item {
        DocItemOrLegacy::LegacyDocItem(v) => {
            if let Some(Expr::Lambda(l)) = entry.value() {
                Some(LegacyDocItem {
                    args: collect_lambda_args_legacy(l),
                    ..v
                })
            } else {
                Some(v)
            }
        }
        // Convert DocItems into legacyItem for markdown rendering
        DocItemOrLegacy::DocItem(v) => Some(LegacyDocItem {
            args: vec![],
            name: v.name,
            comment: v.comment,
        }),
    }
}

// a binding is an assignment, which can take place in an attrset
// - as attributes
// - as inherits
fn collect_bindings(
    node: &SyntaxNode,
    prefix: &str,
    category: &str,
    locs: &HashMap<String, String>,
    scope: HashMap<String, ManualEntry>,
) -> Vec<ManualEntry> {
    for ev in node.preorder() {
        match ev {
            WalkEvent::Enter(n) if n.kind() == SyntaxKind::NODE_ATTR_SET => {
                let mut entries = vec![];
                for child in n.children() {
                    if let Some(apv) = AttrpathValue::cast(child.clone()) {
                        entries.extend(
                            collect_entry_information(apv)
                                .map(|di| di.into_entry(prefix, category, locs)),
                        );
                    } else if let Some(inh) = Inherit::cast(child) {
                        // `inherit (x) ...` needs much more handling than we can
                        // reasonably do here
                        if inh.from().is_some() {
                            continue;
                        }
                        entries.extend(inh.attrs().filter_map(|a| match a {
                            Attr::Ident(i) => scope.get(&i.syntax().text().to_string()).cloned(),
                            // ignore non-ident keys. these aren't useful as lib
                            // functions in general anyway.
                            _ => None,
                        }));
                    }
                }
                return entries;
            }
            _ => (),
        }
    }

    vec![]
}

/// Given a let-in expression and an identifier name, find the corresponding
/// AttrpathValue binding in the let block.
fn find_let_binding(let_in: &LetIn, name: &str) -> Option<AttrpathValue> {
    for entry in let_in.entries() {
        if let Some(apv) = AttrpathValue::cast(entry.syntax().clone()) {
            if let Some(path) = apv.attrpath() {
                // Check if this binding matches the identifier we're looking for
                if path.to_string() == name {
                    return Some(apv);
                }
            }
        }
    }
    None
}

/// Resolve an identifier in the context of a let-in expression.
/// Returns the syntax node of the value if found, following chains of identifiers.
fn resolve_let_ident(let_in: &LetIn, ident: &Ident) -> Option<SyntaxNode> {
    let name = ident.to_string();
    let apv = find_let_binding(let_in, &name)?;
    let value = apv.value()?;

    // If the value is itself an identifier, follow the chain
    if let Expr::Ident(ref inner_ident) = value {
        resolve_let_ident(let_in, inner_ident)
    } else {
        Some(value.syntax().clone())
    }
}

// Main entrypoint for collection
// TODO: document
fn collect_entries(
    root: rnix::Root,
    prefix: &str,
    category: &str,
    locs: &HashMap<String, String>,
    export: &Option<Vec<String>>,
) -> Vec<ManualEntry> {
    // we will look into the top-level let and its body for function docs.
    // we only need a single level of scope for this.
    // since only the body can export a function we don't need to implement
    // mutually recursive resolution.
    let mut preorder = root.syntax().preorder();
    while let Some(ev) = preorder.next() {
        match ev {
            // Skip patterns. See test/patterns.nix for the reason why.
            WalkEvent::Enter(n) if n.kind() == SyntaxKind::NODE_PATTERN => {
                preorder.skip_subtree();
            }
            WalkEvent::Enter(n) if n.kind() == SyntaxKind::NODE_LET_IN => {
                let let_in = LetIn::cast(n.clone()).unwrap();
                let scope: HashMap<String, ManualEntry> = n
                    .children()
                    .filter_map(AttrpathValue::cast)
                    .filter_map(collect_entry_information)
                    .map(|di| (di.name.to_string(), di.into_entry(prefix, category, locs)))
                    .collect();

                // If export list is provided, return only those bindings from the let block
                if let Some(ref exports) = export {
                    return exports
                        .iter()
                        .filter_map(|name| scope.get(name).cloned())
                        .collect();
                }

                // Get the body expression
                let body = let_in.body().unwrap();

                // Check if the body is just an identifier reference
                if let Expr::Ident(ref ident) = body {
                    // Try to resolve the identifier to a let binding
                    if let Some(resolved) = resolve_let_ident(&let_in, ident) {
                        return collect_bindings(&resolved, prefix, category, locs, scope);
                    }
                }

                // Default behavior: look for attrset in the body
                return collect_bindings(body.syntax(), prefix, category, locs, scope);
            }
            WalkEvent::Enter(n) if n.kind() == SyntaxKind::NODE_ATTR_SET => {
                return collect_bindings(&n, prefix, category, locs, Default::default());
            }
            _ => (),
        }
    }

    vec![]
}

/// Extract just the file-level documentation comment from a Nix file.
/// Returns None if no file-level doc comment is found.
fn extract_file_doc(nix: &rnix::Root) -> Option<String> {
    nix.syntax()
        .first_child()
        .and_then(|node| {
            retrieve_doc_comment(&node, Some(0)).or(retrieve_legacy_comment(&node, false))
        })
        .and_then(|doc_item| handle_indentation(&doc_item))
}

fn retrieve_description(nix: &rnix::Root, description: &str, category: &str) -> String {
    if description.is_empty() && category.is_empty() {
        return String::new();
    }
    format!(
        "# {} {{#sec-functions-library-{}}}\n{}\n",
        description,
        category,
        extract_file_doc(nix).unwrap_or_default()
    )
}

fn main_with_options(opts: LegacyOptions) -> String {
    let src = fs::read_to_string(&opts.file).unwrap();
    let locs = match opts.locs {
        None => Default::default(),
        Some(p) => fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .expect("could not read location information"),
    };
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let description = retrieve_description(&nix, &opts.description, &opts.category);

    let entries = collect_entries(nix, &opts.prefix, &opts.category, &locs, &opts.export);

    if opts.json_output {
        let json_string = match serde_json::to_string(&JsonFormat {
            version: 1,
            entries,
        }) {
            Ok(json) => json,
            Err(error) => panic!("Problem converting entries to JSON: {error:?}"),
        };
        json_string
    } else {
        // TODO: move this to commonmark.rs
        let mut output = description + "\n";

        for entry in entries {
            entry.write_section(opts.anchor_prefix.as_str(), &mut output);
        }
        output
    }
}

fn main() {
    let args = Args::parse();

    match args.command {
        Some(Command::Options {
            file,
            output,
            title,
            preamble,
            anchor_prefix,
            include_declarations,
            declarations_base_url,
            revision,
        }) => {
            // New options rendering mode
            let render_opts = options::RenderOptions {
                anchor_prefix,
                include_declarations,
                declarations_base_url,
                revision,
            };

            let parsed = options::parse_options_file(&file).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

            let result = options::render_options_document(
                &parsed,
                &title,
                preamble.as_deref(),
                &render_opts,
            );

            if let Some(out_path) = output {
                fs::write(&out_path, &result).unwrap_or_else(|e| {
                    eprintln!("Error writing output: {}", e);
                    std::process::exit(1);
                });
            } else {
                println!("{}", result);
            }
        }
        Some(Command::FileDoc {
            file,
            format,
            shift_headings: shift_amount,
        }) => {
            // Extract file-level documentation
            let src = fs::read_to_string(&file).unwrap_or_else(|e| {
                eprintln!("Error reading file: {}", e);
                std::process::exit(1);
            });
            let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");

            let doc = extract_file_doc(&nix).map(|d| {
                if shift_amount > 0 {
                    shift_headings(&d, shift_amount)
                } else {
                    d
                }
            });

            match format.as_str() {
                "json" => {
                    let json_obj = serde_json::json!({
                        "file": file.to_string_lossy(),
                        "doc": doc
                    });
                    println!("{}", serde_json::to_string_pretty(&json_obj).unwrap());
                }
                "plain" => {
                    if let Some(d) = doc {
                        println!("{}", d);
                    }
                }
                "markdown" | _ => {
                    if let Some(d) = doc {
                        // For markdown, output as-is (already formatted)
                        println!("{}", d);
                    }
                }
            }
        }
        None => {
            // Legacy mode - require category, description, and file
            let category = args.category.unwrap_or_else(|| {
                eprintln!("Error: --category is required in legacy mode");
                eprintln!("Hint: Use 'nixdoc options --file <file>' for options rendering");
                std::process::exit(1);
            });
            let description = args.description.unwrap_or_else(|| {
                eprintln!("Error: --description is required in legacy mode");
                std::process::exit(1);
            });
            let file = args.file.unwrap_or_else(|| {
                eprintln!("Error: --file is required in legacy mode");
                std::process::exit(1);
            });

            let opts = LegacyOptions {
                prefix: args.prefix,
                anchor_prefix: args.anchor_prefix,
                json_output: args.json_output,
                category,
                description,
                file,
                locs: args.locs,
                export: args.export,
            };
            let output = main_with_options(opts);
            println!("{}", output)
        }
    }
}
