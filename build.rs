use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=data/crucible.gresource.xml");
    println!("cargo:rerun-if-changed=data/icons");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target = format!("{out_dir}/crucible.gresource");

    let status = Command::new("glib-compile-resources")
        .args([
            "--target",
            &target,
            "--sourcedir=data",
            "data/crucible.gresource.xml",
        ])
        .status()
        .expect("glib-compile-resources not found. Install glib2-devel or libglib2.0-dev.");

    if !status.success() {
        panic!("glib-compile-resources failed");
    }
}
