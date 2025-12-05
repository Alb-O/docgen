use rnix;
use std::fs;
use std::path::PathBuf;

use crate::{
    Args, ManualEntry, collect_entries, extract_file_doc, format::shift_headings, main_with_args,
    options, retrieve_description,
};

fn make_args(file: &str, category: &str, description: &str, locs: Option<&str>) -> Args {
    Args {
        command: None,
        prefix: String::from("lib"),
        anchor_prefix: String::from("function-library-"),
        json_output: false,
        category: category.to_string(),
        description: description.to_string(),
        file: Some(PathBuf::from(file)),
        locs: locs.map(PathBuf::from),
        export: None,
    }
}

#[test]
fn test_main() {
    let args = make_args(
        "test/strings.nix",
        "strings",
        "string manipulation functions",
        Some("test/strings.json"),
    );

    let output = main_with_args(&args);

    insta::assert_snapshot!(output);
}

#[test]
fn test_main_minimal() {
    let args = Args {
        command: None,
        prefix: String::new(),
        anchor_prefix: String::new(),
        json_output: false,
        category: String::new(),
        description: String::new(),
        file: Some(PathBuf::from("test/strings.nix")),
        locs: Some(PathBuf::from("test/strings.json")),
        export: None,
    };

    let output = main_with_args(&args);

    insta::assert_snapshot!(output);
}

#[test]
fn test_json_output() {
    let args = Args {
        command: None,
        prefix: String::from("lib"),
        anchor_prefix: String::from("function-library-"),
        json_output: true,
        category: String::from("strings"),
        description: String::from("string manipulation functions"),
        file: Some(PathBuf::from("test/strings.nix")),
        locs: Some(PathBuf::from("test/strings.json")),
        export: None,
    };

    let output = main_with_args(&args);

    insta::assert_snapshot!(output);
}

#[test]
fn test_arg_formatting() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/arg-formatting.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "options";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_inherited_exports() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/inherited-exports.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "let";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_doc_comment() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/doc-comment.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "debug";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_commonmark() {
    let src = fs::read_to_string("test/commonmark.md").unwrap();

    let output = shift_headings(&src, 0);

    insta::assert_snapshot!(output);
}

#[test]
fn test_headings() {
    let src = fs::read_to_string("test/headings.md").unwrap();

    let output = shift_headings(&src, 2);

    insta::assert_snapshot!(output);
}

#[test]
fn test_doc_comment_section_description() {
    let src = fs::read_to_string("test/doc-comment-sec-heading.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "debug";
    let desc = retrieve_description(&nix, &"Debug", category);
    let mut output = String::from(desc) + "\n";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_doc_comment_no_duplicate_arguments() {
    let src = fs::read_to_string("test/doc-comment-arguments.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "debug";
    let desc = retrieve_description(&nix, &"Debug", category);
    let mut output = String::from(desc) + "\n";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_empty_prefix() {
    let test_entry = ManualEntry {
        args: vec![],
        category: "test".to_string(),
        location: None,
        description: vec![],
        example: None,
        fn_type: None,
        name: "mapSimple'".to_string(),
        prefix: "".to_string(),
    };

    let (ident, title) = test_entry.get_ident_title();

    assert_eq!(ident, "test.mapSimple-prime");
    assert_eq!(title, "test.mapSimple'");
}

#[test]
fn test_patterns() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/patterns.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "debug";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_let_ident() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/let-ident.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "math";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_let_ident_chained() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/let-ident-chained.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "math";

    for entry in collect_entries(nix, prefix, category, &Default::default(), &None) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_export_flag() {
    let mut output = String::from("");
    let src = fs::read_to_string("test/export.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");
    let prefix = "lib";
    let category = "export";

    // With --export, we specify which let bindings to document
    let export_list = Some(vec![
        "exportedFunc".to_string(),
        "anotherExported".to_string(),
    ]);

    for entry in collect_entries(nix, prefix, category, &Default::default(), &export_list) {
        entry.write_section("function-library-", &mut output);
    }

    insta::assert_snapshot!(output);
}

#[test]
fn test_options_rendering() {
    let json = fs::read_to_string("test/options.json").unwrap();
    let parsed = options::parse_options_json(&json).unwrap();

    let render_opts = options::RenderOptions {
        anchor_prefix: "opt-".to_string(),
        include_declarations: true,
        declarations_base_url: Some("https://github.com/example/repo".to_string()),
        revision: Some("main".to_string()),
    };

    let output = options::render_options_document(
        &parsed,
        "Module Options",
        Some("These are the available module options."),
        &render_opts,
    );

    insta::assert_snapshot!(output);
}

#[test]
fn test_file_doc_extraction() {
    // Test file-level doc comment extraction
    let src = fs::read_to_string("test/doc-comment-sec-heading.nix").unwrap();
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");

    let doc = extract_file_doc(&nix);
    assert!(doc.is_some());
    insta::assert_snapshot!(doc.unwrap());
}

#[test]
fn test_file_doc_no_doc() {
    // A file without a file-level doc comment
    let src = "{ foo = 1; }";
    let nix = rnix::Root::parse(&src).ok().expect("failed to parse input");

    let doc = extract_file_doc(&nix);
    assert!(doc.is_none());
}
