use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let shaders = manifest.join("shaders");
    println!("cargo:rerun-if-changed={}", shaders.display());

    for (name, stage) in [
        ("gbuffer.vert", "vert"),
        ("gbuffer.frag", "frag"),
        ("lighting.vert", "vert"),
        ("lighting.frag", "frag"),
    ] {
        let src = shaders.join(name);
        let dst = out.join(format!("{name}.spv"));
        println!("cargo:rerun-if-changed={}", src.display());
        let status = Command::new("glslc")
            .arg(format!("-fshader-stage={stage}"))
            .args(["-O", "-o"])
            .arg(&dst)
            .arg(&src)
            .status()
            .expect("glslc (shaderc) must be installed to build the renderer");
        assert!(status.success(), "glslc failed on {name}");
    }
}
