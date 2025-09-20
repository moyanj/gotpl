# `gotpl`

[![Crates.io](https://img.shields.io/crates/v/gotpl.svg)](https://crates.io/crates/gotpl)
[![Docs.rs](https://docs.rs/gotpl/badge.svg)](https://docs.rs/gotpl)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A Rust library for using Go's powerful `text/template` and `html/template` engines via FFI. Get the safety of Rust with the mature templating power of Go.

## ✨ Features

*   **Full Go Template Support**: Complete support for Go's `text/template` and `html/template` syntax.
*   **HTML Safety**: Automatic HTML escaping for XSS prevention via `html/template`.
*   **Flexible Data**: Render templates with any `serde::Serialize`-able Rust data.
*   **Idiomatic Errors**: Go template errors are converted into Rust `Result` types.
*   **Memory Safe**: Memory allocated by Go is safely managed and freed from Rust to prevent leaks.
*   **Lightweight**: Depends only on `serde` for data serialization.

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
gotpl = "0.2.3" # Use the latest version
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Example
```rust
use gotpl::TemplateRenderer;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let template = "Hello, {{.name}}! You have {{len .items}} items.";
    let data = json!({
        "name": "MoYan",
        "items": ["book", "pen"]
    });

    let output = TemplateRenderer::new(template, &data).render()?;

    println!("{}", output);

    Ok(())
}
```

## 🌐 Go Template Syntax

This library supports the complete Go template syntax. For details, see the official docs:

*   [`text/template` Docs](https://pkg.go.dev/text/template)
*   [`html/template` Docs](https://pkg.go.dev/html/template)

## 🛠️ Build Process

`gotpl` uses a `build.rs` script to compile the Go source into a static library and generate Rust FFI bindings with `bindgen`.

**Requirements:**
*   **Go Compiler** (version 1.18+)
*   **Rust Toolchain** (Cargo)

The build is fully automated when you run `cargo build`.

## ⚠️ Caveats

*   **Performance**: FFI calls have overhead. Profile accordingly for high-performance use cases.
*   **Binary Size**: Includes the Go runtime, which will increase your final binary size.
*   **Memory**: All memory is managed safely across the FFI boundary.

## 🤝 Contributing

Contributions are welcome via Pull Requests or Issues.

## 📜 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.