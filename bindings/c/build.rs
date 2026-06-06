//! Generate `include/tharikey.h` from the `extern "C"` surface so the header is always in sync with
//! the Rust ABI. Consumers (Swift module map, C++, JNI) include the generated header.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(&crate_dir).join("include").join("tharikey.h");
    std::fs::create_dir_all(out.parent().unwrap()).ok();

    match cbindgen::generate(&crate_dir) {
        Ok(bindings) => {
            bindings.write_to_file(&out);
        }
        // Don't fail the build if header generation hiccups — the staticlib is still valid.
        Err(e) => println!("cargo:warning=cbindgen header generation failed: {e}"),
    }
}
