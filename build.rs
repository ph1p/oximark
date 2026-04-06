fn main() {
    // md4c is only linked when the "bench-md4c" feature is active and we're
    // not targeting wasm32 (which has no C stdlib).
    if std::env::var("CARGO_FEATURE_BENCH_MD4C").is_err() {
        return;
    }
    let target = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target == "wasm32" {
        return;
    }

    // Use pkg-config to locate the system-installed md4c-html library.
    // Install via: brew install md4c  (macOS) or apt install libmd4c-dev (Linux)
    println!("cargo:rustc-link-lib=md4c-html");
    println!("cargo:rustc-link-lib=md4c");
    if let Ok(out) = std::process::Command::new("pkg-config")
        .args(["--libs-only-L", "md4c-html"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        for part in s.split_whitespace() {
            if let Some(path) = part.strip_prefix("-L") {
                println!("cargo:rustc-link-search=native={path}");
            }
        }
    }
}
