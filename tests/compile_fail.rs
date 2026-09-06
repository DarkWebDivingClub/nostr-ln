//! Things that must not compile.
//!
//! The DoD asks that a consumer "cannot reach a relay, an event kind, a
//! grant or a bucket through the crate's public API — asserted by a
//! compile-fail test, not by inspection". These are that assertion.

#[test]
fn things_that_must_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
