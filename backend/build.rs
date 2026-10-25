// 编译 C++ Core 为静态库并链接进 BackEnd
fn main() {
    let dst = cmake::Config::new("../core")
        .define("SR_CORE_BUILD_SHARED", "OFF")
        .define("SR_CORE_BUILD_TESTS", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=starraft_core");

    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") | Ok("ios") => println!("cargo:rustc-link-lib=c++"),
        Ok("windows") => {}
        _ => println!("cargo:rustc-link-lib=stdc++"),
    }

    println!("cargo:rerun-if-changed=../core/src");
    println!("cargo:rerun-if-changed=../core/include");
    println!("cargo:rerun-if-changed=../core/CMakeLists.txt");
    println!("cargo:rerun-if-changed=../data");
}
