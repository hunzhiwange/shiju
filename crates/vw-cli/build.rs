fn main() {
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var("PROFILE").as_deref() == Ok("debug")
    {
        // Apple's ld documents this option for debug binaries exceeding the
        // 16 MiB compact-unwind offset limit. Keep DWARF unwind tables and panic
        // recovery intact; only omit this debug-only performance diagnostic.
        println!("cargo:rustc-link-arg-bin=shiju=-Wl,-no_warn_eh_frame_too_large");
    }
}
