// SPDX-License-Identifier: LGPL-3.0-or-later
fn main() {
    let glib = pkg_config::Config::new().probe("gio-2.0").unwrap();
    let mut build = cc::Build::new();
    build
        .includes(&glib.include_paths)
        .include("native/chicago")
        .flag("-std=c11")
        .warnings(false);
    for entry in std::fs::read_dir("native/chicago").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|v| v == "c") {
            build.file(path);
        }
    }
    build
        .file("native/common/goodix-crc.c")
        .compile("gxfp_chicago");
    cc::Build::new()
        .includes(&glib.include_paths)
        .include("native")
        .flag("-std=c11")
        .warnings_into_errors(true)
        .file("native/bridge.c")
        .compile("gxfp_chicago_bridge");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rerun-if-changed=native");
}
