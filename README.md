# `gotpl`

[![Crates.io](https://img.shields.io/crates/v/gotpl.svg)](https://crates.io/crates/gotpl)
[![Docs.rs](https://docs.rs/gotpl/badge.svg)](https://docs.rs/gotpl)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

`gotpl` 是一个 Rust 库，它通过 Go 语言的 FFI (Foreign Function Interface) 完整地将 Go 强大的 `text/template` 和 `html/template` 引擎引入 Rust 生态系统。这意味着你可以在 Rust 项目中利用 Go 模板的丰富功能和成熟的生态，同时享受 Rust 的安全和性能。

## ✨ 特性

*   **完整的 Go 模板支持**：在 Rust 中使用 Go 语言原生的 `text/template` 和 `html/template` 语法和功能，包括条件、循环、函数、嵌套模板等。
*   **HTML 安全性**：通过 `html/template` 模式自动进行 HTML 转义，有效防止 XSS 攻击，确保渲染内容的安全性。
*   **灵活的数据绑定**：接受任何实现 `serde::Serialize` trait 的 Rust 数据结构（如 `struct`、`enum`、`serde_json::Value`），自动将其序列化为 JSON 传递给 Go 模板。
*   **清晰的错误处理**：将 Go 模板渲染过程中产生的错误转换为 Rust 的 `Result` 类型，提供详细的错误信息。
*   **零额外依赖**：在 Rust 端仅依赖 `serde` 和 `serde_json` 进行数据序列化，Go 模板引擎是内置的。
*   **内存安全**：通过 FFI 边界的内存管理机制，确保 Go 分配的字符串内存能被 Rust 正确释放，避免内存泄漏。

## 🚀 快速开始

### 安装

将 `gotpl` 添加到你的 `Cargo.toml`：

```toml
[dependencies]
gotpl = "0.1.0" # 替换为最新版本
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 示例

下面是一个简单的例子，展示如何在 Rust 中渲染一个 Go 模板：

```rust
use gotpl::render_template;
use serde::{Serialize};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 示例 1: 渲染一个简单的模板 (带 HTML 转义)
    let template_content = "Hello, {{.Name}}! You are {{.Age}} years old.";
    let data = json!({"Name": "World", "Age": 30});
    let rendered_output = render_template(template_content, &data, true)?;
    println!("Rendered (escaped): {}", rendered_output);
    // 预期输出: Rendered (escaped): Hello, World! You are 30 years old.

    // 示例 2: 使用自定义结构体作为数据源
    #[derive(Debug, Serialize)]
    struct User {
        name: String,
        email: String,
    }

    let user = User {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    let user_template = "User: {{.name}}, Email: {{.email}}";
    let rendered_user = render_template(user_template, &user, true)?;
    println!("Rendered user: {}", rendered_user);
    // 预期输出: Rendered user: User: Alice, Email: alice@example.com

    // 示例 3: 渲染包含潜在 HTML 的模板 (不进行 HTML 转义)
    let html_template = "<h1>{{.Title}}</h1><p>{{.Content}}</p>";
    let html_data = json!({
        "Title": "My Page",
        "Content": "<script>alert('XSS Attack!');</script>"
    });
    // 注意: 这里设置为 `false` 来禁用 HTML 转义，输出原始 HTML 内容。
    // 在生产环境中处理用户生成内容时，请务必谨慎使用。
    let rendered_raw_html = render_template(html_template, &html_data, false)?;
    println!("\nRendered (raw HTML): {}", rendered_raw_html);
    // 预期输出: Rendered (raw HTML): <h1>My Page</h1><p><script>alert('XSS Attack!');</script></p>

    // 示例 4: 渲染包含潜在 HTML 的模板 (进行 HTML 转义，默认行为)
    let rendered_escaped_html = render_template(html_template, &html_data, true)?;
    println!("\nRendered (escaped HTML): {}", rendered_escaped_html);
    // 预期输出: Rendered (escaped HTML): <h1>My Page</h1><p>&lt;script&gt;alert(&#39;XSS Attack!&#39;);&lt;/script&gt;</p>


    // 示例 5: 错误处理 - 模板语法错误
    let invalid_template = "This is {{.AnInvalid.Template.";
    let error_result = render_template(invalid_template, &json!({}), true);
    if let Err(e) = error_result {
        println!("\nError rendering template: {}", e);
        // 预期输出: Error rendering template: Go Template Error: Failed to parse HTML template: ...
    }

    Ok(())
}
```

## 🌐 Go 模板语法

`gotpl` 完全支持 Go 语言的 `text/template` 和 `html/template` 语法。你可以查阅官方文档了解更多细节：

*   [`text/template` 官方文档](https://pkg.go.dev/text/template)
*   [`html/template` 官方文档](https://pkg.go.dev/html/template)

一些常用的 Go 模板语法示例：

```go
// 变量访问
Hello, {{.Name}}!

// 条件语句
{{if .IsAdmin}}Welcome, Admin!{{else}}Welcome, User.{{end}}

// 循环 (迭代 slice 或 map)
<ul>
{{range .Items}}
    <li>{{.}}</li>
{{end}}
</ul>

// 嵌套字段访问
Your address: {{.User.Address.Street}}

// 函数调用 (Go 模板内置函数，例如 len, index, print, printf 等)
Number of items: {{len .Items}}
```

## 🛠️ 构建过程

`gotpl` 内部通过 `go build -buildmode=c-archive` 命令将 Go 代码编译成一个 C 静态库，然后使用 `bindgen` 工具为这个 C 库生成 Rust FFI 绑定。这个过程在 `build.rs` 中自动化完成。

**要求：**
*   **Go 语言环境**: 确保你的系统上安装了 Go 语言编译器 (版本 1.18 或更高)。
*   **Rust 工具链**: 确保安装了 Rust 和 Cargo。

当你运行 `cargo build` 时，`build.rs` 会自动执行以下步骤：
1.  切换到 `src/go_ffi` 目录。
2.  运行 `go build -o ../../target/go_lib/libgo_ffi.a -buildmode=c-archive ffi.go` 将 Go 代码编译为静态库。
3.  使用 `bindgen` 从 `src/go_ffi/ffi.go` 中的 Cgo 注释生成 Rust 绑定。
4.  将生成的绑定文件放置在 `OUT_DIR` 中，以便 `lib.rs` 可以 `include!` 它。

## ⚠️ 注意事项

*   **性能考量**：FFI 调用会带来一定的开销。对于需要极高性能渲染的场景，可能需要评估 Go 模板的适用性。
*   **Go 运行时**：虽然将 Go 代码编译为 C 静态库，但它仍然包含 Go 运行时。这意味着你的二进制文件会略微增大。
*   **内存管理**：Go 模板渲染的结果字符串是在 Go 运行时中分配的 C 字符串。`gotpl` 确保在 Rust 端使用完毕后，通过 `FreeResultString` 函数将这些内存安全地交还给 Go 运行时释放，防止内存泄漏。

## 🤝 贡献

欢迎通过 Pull Requests 或 Issues 贡献代码、报告 Bug 或提出功能建议。

## 📜 许可证

`gotpl` 采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。