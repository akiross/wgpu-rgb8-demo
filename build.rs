use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=shaders/shader.vert");
    println!("cargo:rerun-if-changed=shaders/shader.frag");

    Command::new("glslangValidator")
        .args(&["-V", "shaders/shader.vert", "-o", "shaders/shader.vert.spv"])
        .status()
        .unwrap();
    Command::new("glslangValidator")
        .args(&["-V", "shaders/shader.frag", "-o", "shaders/shader.frag.spv"])
        .status()
        .unwrap();
}
