fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH");
    if let Ok(arch) = arch {
        if arch == *"aarch64" {
            println!("cargo:rustc-cfg=rpi");
        }
    }
}
