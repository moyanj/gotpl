extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 如果是文档构建，则跳过 Go 编译
    if env::var("DOCS").is_ok() {
        println!("cargo:warning=Skipping Go compilation for documentation build.");
        return;
    }

    // 告诉 Cargo 如果 go_ffi/ffi.go 发生变化，就重新运行 build.rs
    println!("cargo:rerun-if-changed=src/go_ffi/ffi.go");

    // 获取 Cargo 的 OUT_DIR (输出目录)
    let lib_out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 1. 编译 Go 模块为 C 静态库和头文件
    let mut cmd = Command::new("go");
    cmd.current_dir("src/go_ffi") // 设置 Go 编译的当前工作目录
        .envs(env::vars()) // 将所有环境变量传递给 Go 命令
        .args(&[
            "build",
            "--buildmode=c-archive", // 编译为 C 静态库
            "-o",
            // 输出文件名为 libgoffi.a 和 libgoffi.h
            format!("{}/{}", lib_out_path.display(), "libgoffi.a").as_str(),
        ]);

    let status = cmd.status().expect("Failed to execute Go build command");
    assert!(
        status.success(),
        "Go build failed with status: {:?}",
        status
    );
    println!(
        "Go build successful. Output: libgoffi.a and libgoffi.h in {}",
        lib_out_path.display()
    );

    // 2. 告诉 Rust 链接器在哪里查找 Go 编译生成的库
    println!("cargo:rustc-link-search={}", lib_out_path.display());
    // 告诉 Rust 链接器链接名为 "goffi" 的静态库 (对应 libgoffi.a)
    println!("cargo:rustc-link-lib=static=goffi");

    // 3. 使用 bindgen 为 Go 生成的 C 头文件创建 Rust 绑定
    let bindings = bindgen::Builder::default()
        // 指定 Go 编译生成的头文件
        .header(format!("{}/{}", lib_out_path.display(), "libgoffi.h").as_str())
        // 自动将 Cargo 特殊的 `rerun-if-changed` 指令添加到生成的绑定中
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // 4. 将生成的 Rust 绑定写入到 OUT_DIR 中的文件
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_file = out_path.join("api_bindings.rs");
    bindings
        .write_to_file(&bindings_file)
        .expect("Couldn't write bindings!");
    println!("Rust bindings generated to {}", bindings_file.display());
}
