//! Produces `vectors/nwc.json` **by reading the specifications**.
//!
//! Same discipline as `generate_nnc_vectors`: these are the documents' own
//! examples, not examples written to match the code. A type that only
//! round-trips what it emits proves nothing.
//!
//! Two sources, because NIP-47 is modular since 2026-08-01 — a five-method
//! core plus numbered extension specifications:
//!
//! ```sh
//! git -C ~/git/nips show upstream/master:47.md > /tmp/nip47-core.md
//! cargo run --example generate_nwc_vectors -- \
//!     /tmp/nip47-core.md ~/git/nips/nwc-onchain.md > vectors/nwc.json
//! ```
//!
//! Two differences from the NNC generator, both properties of the upstream
//! document rather than choices:
//!
//! - it fences examples as `yaml` where `XX.md` uses `jsonc`
//! - it writes `"created_at": unixtimestamp` — a bare placeholder, not a
//!   value, so it is not JSON at all. Substituted with `0` here, and the
//!   substitution is named so nobody mistakes the vector for something the
//!   document literally contains

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: generate_nwc_vectors <spec.md> [<spec.md> ...]");
        std::process::exit(1);
    }

    let mut out: Vec<serde_json::Value> = Vec::new();
    for path in &paths {
        let doc = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));
        for (name, body) in sections(&doc, "### `") {
            let mut v = serde_json::Map::new();
            v.insert("name".into(), name.clone().into());
            v.insert("kind".into(), "method".into());
            v.insert("source".into(), path.clone().into());
            if let Some(j) = labelled_block(&body, "Request:") {
                v.insert("request".into(), j);
            }
            if let Some(j) = labelled_block(&body, "Response:") {
                v.insert("response".into(), j);
            }
            if v.contains_key("request") || v.contains_key("response") {
                out.push(v.into());
            }
        }
    }

    let doc_out = serde_json::json!({
        "note": "Generated from the NWC specifications by \
                 examples/generate_nwc_vectors.rs. Each entry is JSON a \
                 specification itself shows, except that the bare token \
                 `unixtimestamp` is substituted with 0 because it is a \
                 placeholder rather than a value.",
        "sources": paths,
        "vectors": out,
    });
    println!("{}", serde_json::to_string_pretty(&doc_out).unwrap());
}

/// Split the document on a heading prefix, returning (name, body).
fn sections(doc: &str, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for chunk in doc.split(prefix).skip(1) {
        let Some(end) = chunk.find('`') else { continue };
        out.push((chunk[..end].to_string(), chunk[end..].to_string()));
    }
    out
}

/// The first fenced block after a label, as JSON.
fn labelled_block(body: &str, label: &str) -> Option<serde_json::Value> {
    let after = body.split(label).nth(1)?;
    let fenced = after.split("```").nth(1)?;
    let inner = ["yaml", "jsonc", "json"]
        .iter()
        .find_map(|tag| fenced.strip_prefix(tag))?;

    let cleaned: String = inner
        .lines()
        .map(|l| match l.find("//") {
            // Only strip a comment that is not inside a string.
            Some(i) if l[..i].matches('"').count() % 2 == 0 => &l[..i],
            _ => l,
        })
        .collect::<Vec<_>>()
        .join("\n");

    // `unixtimestamp` is a placeholder the document writes where a number
    // belongs. Nothing else in these specs is unquoted-non-JSON.
    let cleaned = cleaned.replace("unixtimestamp", "0");
    let cleaned = trailing_commas(&cleaned);
    serde_json::from_str(&cleaned).ok()
}

/// jsonc and yaml tolerate a trailing comma before a close; json does not.
fn trailing_commas(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut res = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        res.push(chars[i]);
        i += 1;
    }
    res
}
