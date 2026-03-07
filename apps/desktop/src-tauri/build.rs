fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.0");

        // ggml-metal Objective-C code uses @available which emits calls to
        // ___isPlatformVersionAtLeast.  That symbol lives in libclang_rt.osx.a
        // which Rust's linker skips because it passes -nodefaultlibs.
        // Ask clang where its runtime dir is and link it explicitly.
        if let Ok(out) = std::process::Command::new("clang")
            .arg("--print-runtime-dir")
            .output()
        {
            let dir = String::from_utf8_lossy(&out.stdout);
            let dir = dir.trim();
            if !dir.is_empty() {
                println!("cargo:rustc-link-search={dir}");
                println!("cargo:rustc-link-lib=static=clang_rt.osx");
            }
        }
    }
    tauri_build::build()
}
