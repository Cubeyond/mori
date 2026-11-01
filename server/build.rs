//! Track embedded migration additions on stable Rust.

/// Rebuild the migration bundle whenever the migration directory changes.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
