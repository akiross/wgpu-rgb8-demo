use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/shader.vert");
    println!("cargo:rerun-if-changed=src/shader.frag");

    Command::new("glslangValidator").args(&["-V", "src/shader.vert", "-o", "src/shader.vert.spv"]).status().unwrap();
    Command::new("glslangValidator").args(&["-V", "src/shader.frag", "-o", "src/shader.frag.spv"]).status().unwrap();
}
