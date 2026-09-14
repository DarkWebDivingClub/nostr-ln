//! Produces `vectors/nwc.json` **by reading the specifications**.
//!
//! Same discipline as `generate_nnc_vectors`: these are the documents' own
//! examples, not examples written to match the code. A type that only
//! round-trips what it emits proves nothing.
//!
//! Many sources, because NIP-47 is modular since 2026-08-01 — a
//! five-method core plus numbered extension specifications, and we adopt
//! some of those and draft others.
//!
//! **Every source is a durable path in a checked-out repository.** It was
//! not always: the first generation read upstream from a session
//! scratchpad that no longer exists, and recorded that path in each
//! vector's `source`, so `vectors/nwc.json` could not be regenerated at
//! all. Mission 25.2 cloned `github.com/nostr-wallet-connect/nwc` to
//! `~/git/nwc` to fix it. A vector whose provenance cannot be re-read is
//! a vector nobody can check.
//!
//! `47.md` is read from `~/git/nips` directly. It used to be fetched from
//! an `upstream` remote that was never configured — so that command could
//! not have worked — and mission 25.1 made the indirection pointless by
//! putting upstream's core there verbatim.
//!
//! ```sh
//! cargo run --example generate_nwc_vectors -- \
//!     ~/git/nips/47.md \
//!     ~/git/nwc/03.md ~/git/nwc/04.md ~/git/nwc/05.md \
//!     ~/git/nips/nwc-onchain.md ~/git/nips/nwc-invoices.md \
//!     ~/git/nips/nwc-bip321.md ~/git/nips/nwc-route.md \
//!     > vectors/nwc.json
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
        for (name, body) in sections(&doc) {
            let mut v = serde_json::Map::new();
            v.insert("name".into(), name.clone().into());
            v.insert("source".into(), path.clone().into());
            if let Some(j) = labelled_block(&body, "Request:") {
                v.insert("request".into(), j);
            }
            if let Some(j) = labelled_block(&body, "Response:") {
                v.insert("response".into(), j);
            }
            // A notification has neither, and until 25.2 that meant it had
            // no vector at all — so nothing checked a notification payload
            // against the document that defines it. `hold_invoice_accepted`
            // was missing `metadata` for exactly as long.
            if let Some(j) = labelled_block(&body, "Notification:") {
                v.insert("notification".into(), j);
            }
            let kind = if v.contains_key("notification") { "notification" } else { "method" };
            v.insert("kind".into(), kind.into());
            if v.contains_key("request")
                || v.contains_key("response")
                || v.contains_key("notification")
            {
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

/// Every backticked `##` or `###` heading, with the body under it.
///
/// Line-anchored rather than a substring split: `"### `"` contains
/// `"## `"`, so splitting on the shorter prefix would find every level-3
/// heading twice and emit each vector twice.
///
/// Both levels because the documents disagree — NWC-02 writes its
/// notifications as `##` and NWC-03 writes its as `###`, and a generator
/// that read one level silently skipped the other.
fn sections(doc: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in doc.lines() {
        let heading = line
            .strip_prefix("### `")
            .or_else(|| line.strip_prefix("## `"))
            .and_then(|rest| rest.split('`').next())
            .filter(|name| !name.is_empty());
        match heading {
            Some(name) => {
                if let Some(done) = current.take() {
                    out.push(done);
                }
                current = Some((name.to_string(), String::new()));
            }
            None => {
                if let Some((_, body)) = current.as_mut() {
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }
    }
    if let Some(done) = current {
        out.push(done);
    }
    out
}

/// The first fenced block after a label, as JSON.
/// The first fenced block under a line introducing `label`.
///
/// The label is matched as a **prefix of a line ending in a colon**, not as
/// an exact string. NWC-12 writes `Request by offer ID and payment hash:`,
/// and an exact match skipped it — so a request the document plainly shows
/// was never decoded by anything.
fn labelled_block(body: &str, label: &str) -> Option<serde_json::Value> {
    // Callers pass `"Request:"`; the colon is the terminator, not part of
    // the word, or `Request by offer ID...:` would not match.
    let label = label.trim_end_matches(':');
    let start = body.lines().enumerate().find_map(|(i, l)| {
        let l = l.trim();
        (l.starts_with(label) && l.ends_with(':')).then_some(i)
    })?;
    let after: String = body.lines().skip(start + 1).collect::<Vec<_>>().join("\n");
    let after = after.as_str();
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

    // Bare placeholders the documents write where a number belongs. The
    // list used to be one entry with a comment claiming nothing else in
    // these specifications was unquoted-non-JSON. `blocknumber` was, in
    // NWC-03, and the claim was written by someone who had read one
    // document.
    let cleaned = cleaned.replace("unixtimestamp", "0").replace("blocknumber", "0");
    let cleaned = trailing_commas(&cleaned);

    // **Loud, not `.ok()`.** A labelled block that does not parse is a
    // placeholder we have not seen or a defect in the document, and
    // swallowing it drops a vector silently — which is exactly what
    // happened: `hold_invoice_accepted` had no vector from the day NWC-03
    // was adopted, because `blocknumber` failed here and nothing said so.
    // A generator that quietly produces fewer vectors is worse than one
    // that stops.
    match serde_json::from_str(&cleaned) {
        Ok(v) => Some(v),
        Err(e) => panic!("a labelled block does not parse as JSON: {e}\n{cleaned}"),
    }
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
