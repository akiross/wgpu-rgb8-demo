use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=./shaders/shader.vert");
    println!("cargo:rerun-if-changed=./shaders/shader.frag");

    let out_dir = std::env::var("OUT_DIR").unwrap();

    let v_shader = format!("{}/shader.vert.spv", out_dir);
    let f_shader = format!("{}/shader.frag.spv", out_dir);

    Command::new("glslangValidator")
        .args(&["-V", "./shaders/shader.vert", "-o", &v_shader])
        .status()
        .unwrap();
    Command::new("glslangValidator")
        .args(&["-V", "./shaders/shader.frag", "-o", &f_shader])
        .status()
        .unwrap();
}
