use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    if target == "arm-unknown-linux-gnueabihf" {
        println!("cargo:rustc-env=PKG_CONFIG_ALLOW_CROSS=1");
        println!("cargo:rustc-link-search=target/mupdf_wrapper/Kobo");
        println!("cargo:rustc-link-search=target/collatinus_wrapper/Kobo");
        println!("cargo:rustc-link-search=libs");
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=Qt5Core");
    } else {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        match target_os.as_ref() {
            "linux" => {
                println!("cargo:rustc-link-search=target/mupdf_wrapper/Linux");
                println!("cargo:rustc-link-search=target/collatinus_wrapper/Linux");
                println!("cargo:rustc-link-lib=dylib=stdc++");
                println!("cargo:rustc-link-lib=dylib=Qt5Core");
            },
            "macos" => {
                println!("cargo:rustc-link-search=target/mupdf_wrapper/Darwin");
                println!("cargo:rustc-link-search=target/collatinus_wrapper/Darwin");
                println!("cargo:rustc-link-lib=dylib=c++");
                // Qt5Core is a framework on macOS
                let qt_lib = std::process::Command::new("brew")
                    .args(["--prefix", "qt@5"])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                if !qt_lib.is_empty() {
                    println!("cargo:rustc-link-search=framework={}/lib", qt_lib);
                }
                println!("cargo:rustc-link-lib=framework=QtCore");
            },
            _ => panic!("Unsupported platform: {}.", target_os),
        }
        println!("cargo:rustc-link-lib=mupdf-third");
    }

    println!("cargo:rustc-link-lib=collatinus_wrapper");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=bz2");
    println!("cargo:rustc-link-lib=jpeg");
    println!("cargo:rustc-link-lib=png16");
    println!("cargo:rustc-link-lib=gumbo");
    println!("cargo:rustc-link-lib=openjp2");
    println!("cargo:rustc-link-lib=jbig2dec");
}
