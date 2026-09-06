//! Produces `vectors/nnc.json` **by reading `XX.md`**.
//!
//! Every `jsonc` block under a `#### \`method\`` heading is an example the
//! specification asserts. Generating from the document is the only way the
//! vectors stay true when it changes — and it changed four times in the
//! week these types were written.
//!
//! ```sh
//! cargo run --example generate_nnc_vectors -- ~/git/nips/XX.md > vectors/nnc.json
//! ```
//!
//! jsonc is not json: the spec's blocks carry `// comments`, which are
//! stripped here rather than in the spec, because the comments are what
//! make the document readable.

use std::collections::BTreeMap;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: generate_nnc_vectors <path to XX.md>");
        std::process::exit(1);
    });
    let doc = std::fs::read_to_string(&path).expect("read XX.md");

    let mut out: Vec<serde_json::Value> = Vec::new();
    for (name, body) in sections(&doc, "#### `") {
        let mut v = serde_json::Map::new();
        v.insert("name".into(), name.clone().into());
        v.insert("kind".into(), "method".into());
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
    for (name, body) in sections(&doc, "### `") {
        if !name.starts_with("channel_") {
            continue;
        }
        if let Some(j) = labelled_block(&body, "Notification:") {
            out.push(serde_json::json!({
                "name": name, "kind": "notification", "notification": j
            }));
        }
    }

    let doc_out = serde_json::json!({
        "note": "Generated from XX.md by examples/generate_nnc_vectors.rs. \
                 Each entry is JSON the specification itself shows.",
        "source": path,
        "vectors": out,
    });
    println!("{}", serde_json::to_string_pretty(&doc_out).unwrap());
}

/// Split the document on a heading prefix, returning (name, body).
fn sections(doc: &str, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut it = doc.split(prefix).skip(1);
    while let Some(chunk) = it.next() {
        let Some(end) = chunk.find('`') else { continue };
        let name = chunk[..end].to_string();
        let body = chunk[end..].to_string();
        out.push((name, body));
    }
    out
}

/// The first fenced block after a label, with jsonc comments stripped.
fn labelled_block(body: &str, label: &str) -> Option<serde_json::Value> {
    let after = body.split(label).nth(1)?;
    let fenced = after.split("```").nth(1)?;
    let inner = fenced.strip_prefix("jsonc").or_else(|| fenced.strip_prefix("json"))?;
    let cleaned: String = inner
        .lines()
        .map(|l| match l.find("//") {
            // Only strip a comment that is not inside a string.
            Some(i) if l[..i].matches('"').count() % 2 == 0 => &l[..i],
            _ => l,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let cleaned = cleaned.replace(",\n}", "\n}").replace(",\n]", "\n]");
    let cleaned = trailing_commas(&cleaned);
    serde_json::from_str(&cleaned).ok()
}

/// jsonc tolerates a trailing comma before a closing brace; json does not.
fn trailing_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending: Option<usize> = None;
    for (i, c) in s.char_indices() {
        match c {
            ',' => pending = Some(i),
            c if c.is_whitespace() => {}
            '}' | ']' => pending = None,
            _ => pending = None,
        }
        let _ = i;
        out.push(c);
    }
    // simple two-pass: drop any comma followed only by whitespace then } or ]
    let bytes: Vec<char> = out.chars().collect();
    let mut res = String::with_capacity(out.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ',' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == '}' || bytes[j] == ']') {
                i += 1;
                continue;
            }
        }
        res.push(bytes[i]);
        i += 1;
    }
    let _ = pending;
    let _: BTreeMap<(), ()> = BTreeMap::new();
    res
}
