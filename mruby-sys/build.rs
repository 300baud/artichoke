#![deny(clippy::all, clippy::pedantic)]
#![deny(warnings, intra_doc_link_resolution_failure)]
#![doc(deny(warnings))]

use fs_extra::dir::{self, CopyOptions};
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// vendored mruby version
const MRUBY_REVISION: &str = "b0786f62";

/// Path helpers
struct Build;

impl Build {
    fn root() -> PathBuf {
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("mruby-sys")
    }

    fn gems() -> Vec<&'static str> {
        vec![
            "mruby-compiler",     // Ruby parser and bytecode generation
            "mruby-error",        // `mrb_raise`, `mrb_protect`
            "mruby-eval",         // eval, instance_eval, and friends
            "mruby-metaprog",     // APIs on Kernel and Module for accessing classes and variables
            "mruby-method",       // `Method`, `UnboundMethod`, and method APIs on Kernel and Module
            "mruby-toplevel-ext", // expose API for top self
            "mruby-enumerator",   // Enumerator class from core
            "mruby-enum-lazy",    // Enumerable#lazy
            "mruby-fiber",        // Fiber class from core, required by mruby-enumerator
            "mruby-pack",         // Array#pack and String#unpack
            "mruby-sprintf",      // Kernel#sprintf, Kernel#format, String#%
            "mruby-class-ext",    // Pending removal, see GH-32
            "mruby-kernel-ext",   // Pending removal, see GH-32
            "mruby-proc-ext",     // required by mruby-method, see GH-32
        ]
    }

    fn build_config() -> PathBuf {
        Build::root().join("build_config.rb")
    }

    fn ext_source_dir() -> PathBuf {
        Build::root().join("mruby-sys")
    }

    fn ext_include_dir() -> PathBuf {
        Build::ext_source_dir().join("include")
    }

    fn ext_source_file() -> PathBuf {
        Build::ext_source_dir()
            .join("src")
            .join("mruby-sys")
            .join("ext.c")
    }

    fn wasm_include_dir() -> PathBuf {
        Build::root()
            .join("vendor")
            .join("emscripten")
            .join("system")
            .join("include")
            .join("libc")
    }

    fn mruby_source_dir() -> PathBuf {
        Build::root().join("vendor").join("mruby")
    }

    fn mruby_minirake() -> PathBuf {
        Build::mruby_source_dir().join("minirake")
    }

    fn mruby_include_dir() -> PathBuf {
        Build::mruby_source_dir().join("include")
    }

    fn mruby_build_dir() -> PathBuf {
        Build::root().join("mruby-build")
    }

    fn mruby_generated_source_dir() -> PathBuf {
        Build::mruby_build_dir().join("sys")
    }

    fn bindgen_source_header() -> PathBuf {
        Build::ext_include_dir().join("mruby-sys.h")
    }
}

fn main() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH");
    let opts = CopyOptions::new();
    let _ = dir::remove(Build::root());
    dir::copy(
        env::var("CARGO_MANIFEST_DIR").unwrap(),
        env::var("OUT_DIR").unwrap(),
        &opts,
    )
    .unwrap();

    let mut gembox = String::from("MRuby::GemBox.new { |conf| ");
    for gem in Build::gems() {
        gembox.push_str("conf.gem core: '");
        gembox.push_str(gem);
        gembox.push_str("';");
    }
    gembox.push('}');
    fs::write(Build::root().join("sys.gembox"), gembox).unwrap();

    // Build the mruby static library with its built in minirake build system.
    // minirake dynamically generates some c source files so we can't build
    // directly with the `cc` crate.
    env::set_var("MRUBY_REVISION", MRUBY_REVISION);
    println!("cargo:rustc-env=MRUBY_REVISION={}", MRUBY_REVISION);
    println!("cargo:rerun-if-env-changed=MRUBY_REVISION");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!(
        "cargo:rerun-if-changed={}",
        Build::build_config().to_string_lossy()
    );
    if !Command::new(Build::mruby_minirake())
        .arg("--jobs")
        .arg("4")
        .env("MRUBY_BUILD_DIR", Build::mruby_build_dir())
        .env("MRUBY_CONFIG", Build::build_config())
        .current_dir(Build::mruby_source_dir())
        .status()
        .unwrap()
        .success()
    {
        panic!("Failed to build generate mruby C sources");
    }

    let mut sources = HashMap::new();
    sources.insert(Build::ext_source_file(), Build::ext_source_file());
    let walker = WalkDir::new(Build::mruby_source_dir()).into_iter();
    for entry in walker {
        if let Ok(entry) = entry {
            let source = entry.path();
            let relative_source = source.strip_prefix(Build::mruby_source_dir()).unwrap();
            let mut is_buildable = source
                .strip_prefix(Build::mruby_source_dir().join("src"))
                .is_ok();
            for gem in Build::gems() {
                is_buildable |= source
                    .components()
                    .any(|component| component == Component::Normal(OsStr::new(gem)));
            }
            if relative_source
                .components()
                .any(|component| component == Component::Normal(OsStr::new("build")))
            {
                // Skip build artifacts generated by minirake invocation that we
                // do not intend to build.
                is_buildable = false;
            }
            if is_buildable && source.extension().and_then(OsStr::to_str) == Some("c") {
                sources.insert(relative_source.to_owned(), source.to_owned());
            }
        }
    }
    let walker = WalkDir::new(Build::mruby_generated_source_dir()).into_iter();
    for entry in walker {
        if let Ok(entry) = entry {
            let source = entry.path();
            let relative_source = source
                .strip_prefix(Build::mruby_generated_source_dir())
                .unwrap();
            if source.extension().and_then(OsStr::to_str) == Some("c") {
                sources.insert(relative_source.to_owned(), source.to_owned());
            }
        }
    }
    let mrb_int = if let Ok("wasm32") = arch.as_ref().map(String::as_str) {
        "MRB_INT32"
    } else {
        "MRB_INT64"
    };

    // Build the extension library
    let mut build = cc::Build::new();
    build
        .warnings(false)
        .files(sources.values())
        .include(Build::mruby_include_dir())
        .include(Build::ext_include_dir())
        .define("MRB_DISABLE_STDIO", None)
        .define("MRB_UTF8_STRING", None)
        .define(mrb_int, None);

    for gem in Build::gems() {
        let mut dir = "include";
        if gem == "mruby-compiler" {
            dir = "core";
        }
        let gem_include_dir = Build::mruby_source_dir()
            .join("mrbgems")
            .join(gem)
            .join(dir);
        build.include(gem_include_dir);
    }

    if let Ok("wasm32") | Ok("wasm64") = arch.as_ref().map(String::as_str) {
        build.include(Build::wasm_include_dir());
        build.define("MRB_DISABLE_DIRECT_THREADING", None);
        build.define("MRB_API", Some(r#"__attribute__((visibility("default")))"#));
    }

    build.compile("libmrubysys.a");

    println!(
        "cargo:rerun-if-changed={}",
        Build::bindgen_source_header().to_string_lossy()
    );
    let bindings_out_path: PathBuf = PathBuf::from(env::var("OUT_DIR").unwrap()).join("ffi.rs");
    let mut bindgen = bindgen::Builder::default()
        .header(Build::bindgen_source_header().to_string_lossy())
        .clang_arg(format!(
            "-I{}",
            Build::mruby_include_dir().to_string_lossy()
        ))
        .clang_arg(format!("-I{}", Build::ext_include_dir().to_string_lossy()))
        .clang_arg("-DMRB_DISABLE_STDIO")
        .clang_arg("-DMRB_UTF8_STRING")
        .clang_arg(format!("-D{}", mrb_int))
        .whitelist_function("^mrb.*")
        .whitelist_type("^mrb.*")
        .whitelist_var("^mrb.*")
        .whitelist_var("^MRB.*")
        .whitelist_var("^MRUBY.*")
        .whitelist_var("REGEXP_CLASS")
        .rustified_enum("mrb_vtype")
        .rustified_enum("mrb_lex_state_enum")
        .rustified_enum("mrb_range_beg_len")
        .rustfmt_bindings(true)
        // work around warnings caused by cargo doc interpreting Ruby doc blocks
        // as Rust code.
        // See: https://github.com/rust-lang/rust-bindgen/issues/426
        .generate_comments(false);
    if let Ok("wasm32") | Ok("wasm64") = arch.as_ref().map(String::as_str) {
        bindgen = bindgen
            .clang_arg(format!("-I{}", Build::wasm_include_dir().to_string_lossy()))
            .clang_arg("-DMRB_DISABLE_DIRECT_THREADING")
            .clang_arg(r#"-DMRB_API=__attribute__((visibility("default")))"#);
    }
    bindgen
        .generate()
        .expect("Unable to generate mruby bindings")
        .write_to_file(bindings_out_path)
        .expect("Unable to write mruby bindings");
}