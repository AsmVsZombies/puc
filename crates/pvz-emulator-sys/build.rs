use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest_dir.join("vendor");
    let lib = vendor.join("lib");
    let shim = manifest_dir.join("shim");

    if !lib.join("world.cpp").exists() {
        panic!(
            "vendor submodule missing at {} — run:\n  \
             git -c protocol.file.allow=always submodule update --init --recursive",
            vendor.display()
        );
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        // The emulator lib relies on -O3 -DNDEBUG semantics (asserts compiled out);
        // the shim validates input at the boundary, so this is safe across FFI.
        .define("NDEBUG", None)
        // The vendored emulator has many int/size_t comparisons; silence the noise
        // so it doesn't bury the Rust build output.
        .flag_if_supported("-Wno-sign-compare")
        .include(&vendor) // common/, constants/, seml/
        .include(&lib) // world.h
        .include(lib.join("lib")) // rapidjson
        .include(lib.join("system"))
        .include(lib.join("object"))
        .include(lib.join("learning"))
        .include(&shim); // puc_shim.h

    // Emulator library sources (mirrors lib/CMakeLists.txt's aux_source_directory set).
    // main.cpp (standalone debug server) and pybind.cpp (python bindings) are excluded.
    for dir in [
        "object",
        "system",
        "system/zombie",
        "system/projectile",
        "system/plant",
        "learning",
    ] {
        add_cpp_glob(&mut build, &lib.join(dir));
    }
    build.file(lib.join("world.cpp"));

    // The FFI shim: the dispatcher plus one TU per calculator (calc_*.cpp). Each
    // calculator gets its own TU so its relative includes can't collide with a
    // sibling's; the shared seml/common headers are inline, so they merge across
    // TUs at link time.
    add_cpp_glob(&mut build, &shim);

    let is_msvc = build.get_compiler().is_like_msvc();

    // The sources are UTF-8 (Chinese plant/zombie names). MSVC otherwise reads them
    // in the system code page (e.g. 936) and chokes on "newline in constant".
    if is_msvc {
        build.flag("/utf-8");
    }

    build.compile("pvzemu");

    // The cores spawn std::threads; on Unix that pulls in pthread at final link
    // (cc emits the C++ stdlib link automatically). MSVC has threading in the CRT —
    // there is no pthread.lib to link.
    if !is_msvc {
        println!("cargo:rustc-link-lib=dylib=pthread");
    }

    // Rebuild when the shim or any vendored source/header changes.
    println!("cargo:rerun-if-changed={}", shim.display());
    rerun_if_tree_changed(&vendor.join("seml"));
    rerun_if_tree_changed(&vendor.join("common"));
    rerun_if_tree_changed(&vendor.join("constants"));
    rerun_if_tree_changed(&lib);
}

fn add_cpp_glob(build: &mut cc::Build, dir: &Path) {
    let pattern = dir.join("*.cpp");
    let pattern = pattern.to_str().expect("non-UTF8 path");
    for entry in glob::glob(pattern).expect("bad glob pattern").flatten() {
        build.file(entry);
    }
}

// Emits rerun-if-changed for every file under `root` (cc only tracks the .cpp it
// compiles, not the headers they include).
fn rerun_if_tree_changed(root: &Path) {
    for entry in glob::glob(&format!("{}/**/*", root.display())).expect("bad glob").flatten() {
        if entry.is_file() {
            println!("cargo:rerun-if-changed={}", entry.display());
        }
    }
}
