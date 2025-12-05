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

mod comment;
mod commonmark;
mod format;
mod options;
#[cfg(test)]
mod test;

use crate::format::handle_indentation;

use self::comment::get_expr_docs;
use self::commonmark::*;
use format::shift_headings;
use rnix::{
    SyntaxKind, SyntaxNode,
    ast::{Attr, AttrpathValue, Expr, HasEntry, Ident, Inherit, Lambda, LetIn, Param},
};
use rowan::{WalkEvent, ast::AstNode};
use std::fs;

use serde::Serialize;
use std::collections::HashMap;

use clap::Parser;
use std::path::PathBuf;

/// Command line arguments for docgen
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Prefix for the category (e.g. 'lib' or 'utils').
    #[arg(short, long, default_value_t = String::from("lib"))]
    prefix: String,

    #[arg(long, default_value_t = String::from("function-library-"))]
    anchor_prefix: String,

    /// Whether to output JSON.
    #[arg(short, long, default_value_t = false)]
    json_output: bool,

    /// Name of the function category (e.g. 'strings', 'attrsets').
    #[arg(short, long, default_value_t = String::new())]
    category: String,

    /// Description of the function category.
    #[arg(short, long, default_value_t = String::new())]
    description: String,

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

#[derive(Debug)]
struct DocComment {
    /// Primary documentation string.
    doc: String,
}

#[derive(Debug)]
struct DocItem {
    name: String,
    comment: DocComment,
    args: Vec<Argument>,
}

#[derive(Debug, Serialize)]
struct JsonFormat {
    version: u32,
    entries: Vec<ManualEntry>,
}

/// Returns a RFC145 doc-comment if one is present
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

/// Traverse directly chained nix lambdas and collect the identifiers of all lambda arguments.
fn collect_lambda_args(mut lambda: Lambda) -> Vec<Argument> {
    let mut args = vec![];

    loop {
        match lambda.param().unwrap() {
            Param::IdentParam(id) => {
                args.push(Argument::Flat(SingleArg {
                    name: id.to_string(),
                    doc: handle_indentation(
                        &retrieve_doc_comment(id.syntax(), Some(1)).unwrap_or_default(),
                    ),
                }));
            }
            Param::Pattern(pat) => {
                let pattern_vec: Vec<_> = pat
                    .pat_entries()
                    .map(|entry| SingleArg {
                        name: entry.ident().unwrap().to_string(),
                        doc: handle_indentation(
                            &retrieve_doc_comment(entry.syntax(), Some(1)).unwrap_or_default(),
                        ),
                    })
                    .collect();

                args.push(Argument::Pattern(pattern_vec));
            }
        }

        match lambda.body() {
            Some(Expr::Lambda(inner)) => lambda = inner,
            _ => break,
        }
    }

    args
}

/// Transforms an AST node into a `DocItem` if it has a leading
/// documentation comment.
fn retrieve_doc_item(node: &AttrpathValue) -> Option<DocItem> {
    let ident = node.attrpath().unwrap();
    let item_name = ident.to_string();

    let doc_comment = retrieve_doc_comment(node.syntax(), Some(2))?;

    Some(DocItem {
        name: item_name,
        comment: DocComment { doc: doc_comment },
        args: vec![],
    })
}

impl DocItem {
    fn into_entry(
        self,
        prefix: &str,
        category: &str,
        locs: &HashMap<String, String>,
    ) -> ManualEntry {
        let ident = get_identifier(
            &prefix.to_string(),
            &category.to_string(),
            &self.name.to_string(),
        );

        ManualEntry {
            prefix: prefix.to_string(),
            category: category.to_string(),
            location: locs.get(&ident).cloned(),
            name: self.name,
            description: self
                .comment
                .doc
                .split("\n\n")
                .map(|s| s.to_string())
                .collect(),
            fn_type: None,
            example: None,
            args: self.args,
        }
    }
}

/// Traverse the arena from a top-level SetEntry and collect, where
/// possible:
///
/// 1. The identifier of the set entry itself.
/// 2. The attached doc comment on the entry.
/// 3. The argument names of any curried functions.
fn collect_entry_information(entry: AttrpathValue) -> Option<DocItem> {
    let mut doc_item = retrieve_doc_item(&entry)?;

    if let Some(Expr::Lambda(l)) = entry.value() {
        doc_item.args = collect_lambda_args(l);
    }

    Some(doc_item)
}

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
                        if inh.from().is_some() {
                            continue;
                        }
                        entries.extend(inh.attrs().filter_map(|a| match a {
                            Attr::Ident(i) => scope.get(&i.syntax().text().to_string()).cloned(),
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
                if path.to_string() == name {
                    return Some(apv);
                }
            }
        }
    }
    None
}

/// Resolve an identifier in the context of a let-in expression.
fn resolve_let_ident(let_in: &LetIn, ident: &Ident) -> Option<SyntaxNode> {
    let name = ident.to_string();
    let apv = find_let_binding(let_in, &name)?;
    let value = apv.value()?;

    if let Expr::Ident(ref inner_ident) = value {
        resolve_let_ident(let_in, inner_ident)
    } else {
        Some(value.syntax().clone())
    }
}

fn collect_entries(
    root: rnix::Root,
    prefix: &str,
    category: &str,
    locs: &HashMap<String, String>,
    export: &Option<Vec<String>>,
) -> Vec<ManualEntry> {
    let mut preorder = root.syntax().preorder();
    while let Some(ev) = preorder.next() {
        match ev {
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

                if let Some(ref exports) = export {
                    return exports
                        .iter()
                        .filter_map(|name| scope.get(name).cloned())
                        .collect();
                }

                let body = let_in.body().unwrap();

                if let Expr::Ident(ref ident) = body {
                    if let Some(resolved) = resolve_let_ident(&let_in, ident) {
                        return collect_bindings(&resolved, prefix, category, locs, scope);
                    }
                }

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
fn extract_file_doc(nix: &rnix::Root) -> Option<String> {
    nix.syntax()
        .first_child()
        .and_then(|node| retrieve_doc_comment(&node, Some(0)))
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

fn main_with_args(args: &Args) -> String {
    let file = args.file.as_ref().expect("--file is required");
    let src = fs::read_to_string(file).unwrap();
    let locs = match &args.locs {
        None => Default::default(),
        Some(p) => fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            .expect("could not read location information"),
    };
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let description = retrieve_description(&nix, &args.description, &args.category);

    let entries = collect_entries(nix, &args.prefix, &args.category, &locs, &args.export);

    if args.json_output {
        serde_json::to_string(&JsonFormat {
            version: 1,
            entries,
        })
        .expect("Problem converting entries to JSON")
    } else {
        let mut output = description + "\n";
        for entry in entries {
            entry.write_section(args.anchor_prefix.as_str(), &mut output);
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
                        println!("{}", d);
                    }
                }
            }
        }
        None => {
            if args.file.is_none() {
                eprintln!("Error: --file is required");
                std::process::exit(1);
            }
            let output = main_with_args(&args);
            println!("{}", output)
        }
    }
}
