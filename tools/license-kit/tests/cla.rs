//! (5) CLA Assistant workflow: valid YAML that references the CLA document (CLA.md).
//!
//! The CLA workflow is written on its own pyrlyn/infra branch; on a branch without it this
//! test skips with a message instead of failing.

mod common;

use std::fs;

use common::*;
use serde_yaml::Value;

const CLA_DOC: &str = "CLA.md";

/// Workflow files of this checkout that use contributor-assistant/github-action.
fn cla_workflows() -> Vec<(String, String)> {
    let dir = infra_root().join(".github/workflows");
    let mut out: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let p = e.ok()?.path();
            let text = fs::read_to_string(&p).ok()?;
            let name = p.file_name()?.to_string_lossy().into_owned();
            text.contains("contributor-assistant")
                .then_some((name, text))
        })
        .collect();
    out.sort();
    out
}

fn walk<'a>(v: &'a Value, found: &mut Vec<&'a Value>) {
    match v {
        Value::Mapping(m) => {
            if m.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|u| u.contains("contributor-assistant/github-action"))
            {
                found.push(v);
            }
            m.values().for_each(|x| walk(x, found));
        }
        Value::Sequence(s) => s.iter().for_each(|x| walk(x, found)),
        _ => {}
    }
}

#[test]
fn cla_workflow_is_valid_and_points_at_cla_md() {
    let wfs = cla_workflows();
    if wfs.is_empty() {
        eprintln!(
            "SKIP cla: no workflow using contributor-assistant/github-action in .github/workflows \
             on this branch (the CLA workflow lands on its own branch)"
        );
        return;
    }
    for (name, text) in wfs {
        let doc: Value =
            serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("{name} is not valid YAML: {e}"));
        assert!(doc.get("jobs").is_some(), "{name} has no jobs");
        let mut steps = Vec::new();
        walk(&doc, &mut steps);
        assert!(
            !steps.is_empty(),
            "{name}: no contributor-assistant/github-action step"
        );
        for step in steps {
            let uses = step["uses"].as_str().unwrap();
            let pin = uses.split('@').nth(1).unwrap_or_default();
            assert!(
                pin.len() == 40 && pin.chars().all(|c| c.is_ascii_hexdigit()),
                "{name}: `{uses}` not SHA-pinned"
            );
            let doc_path = step["with"]["path-to-document"]
                .as_str()
                .unwrap_or_default();
            // A literal, or an expression whose quoted fallback is the default document.
            let literals: Vec<&str> = if doc_path.contains("${{") {
                doc_path.split('\'').skip(1).step_by(2).collect()
            } else {
                vec![doc_path.trim()]
            };
            assert!(
                literals
                    .iter()
                    .any(|l| *l == CLA_DOC || l.ends_with(&format!("/{CLA_DOC}"))),
                "{name}: path-to-document `{doc_path}` does not point at {CLA_DOC}"
            );
            if let Some(default) =
                doc["on"]["workflow_call"]["inputs"]["document-url"]["default"].as_str()
            {
                assert!(
                    default.ends_with(CLA_DOC),
                    "{name}: document-url default `{default}`"
                );
            }
            assert!(
                infra_root().join(CLA_DOC).is_file(),
                "{name} references {CLA_DOC}, missing in this checkout"
            );
        }
    }
}
