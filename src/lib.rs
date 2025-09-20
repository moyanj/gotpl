// 允许一些 bindgen 生成代码中常见的 lint 警告
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(unused)]
mod bindings {
    // 导入 build.rs 生成的 Rust 绑定
    include!(concat!(env!("OUT_DIR"), "/api_bindings.rs"));
}
// 使用生成的绑定中的类型和函数
use bindings::*;

use std::ffi::{CStr, CString};
use std::fmt;

/// Represents an error that occurred during Go template rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateError(String);

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Go Template Error: {}", self.0)
    }
}

impl std::error::Error for TemplateError {}

/// Renders a Go template with provided data.
///
/// # Arguments
/// * `template_content` - The Go template string.
/// * `data` - The data to be used in the template. This can be any type that implements `serde::Serialize`,
///            such as `serde_json::Value`, structs, or enums. It will be serialized into a JSON string.
/// * `escape_html` - A boolean indicating whether to escape HTML characters in the output.
///                   Set to `true` for security (e.g., preventing XSS), `false` to output raw data.
///
/// # Returns
/// A `Result` indicating success (`String` with rendered output) or failure (`TemplateError`).
pub fn render_template<T: serde::Serialize>(
    template_content: &str,
    data: &T, // 接受任何 Serialize 类型
    escape_html: bool,
) -> Result<String, TemplateError> {
    // 将 Rust 字符串转换为 C 字符串，以便传递给 Go
    let c_template_content = CString::new(template_content).map_err(|e| {
        TemplateError(format!(
            "Failed to convert template content to CString: {}",
            e
        ))
    })?;

    // 将传入的 Rust 数据序列化为 JSON 字符串
    let json_data_string = serde_json::to_string(data)
        .map_err(|e| TemplateError(format!("Failed to serialize data to JSON: {}", e)))?;

    // 将 JSON 字符串转换为 C 字符串
    let c_json_data = CString::new(json_data_string).map_err(|e| {
        TemplateError(format!(
            "Failed to convert JSON data string to CString: {}",
            e
        ))
    })?;

    // 调用 Go 函数。这是不安全的，因为涉及 FFI。
    let result = unsafe {
        RenderTemplate(
            c_template_content.as_ptr() as *mut i8,
            c_json_data.as_ptr() as *mut i8,
            escape_html, // 传递 escape_html 参数
        )
    };

    // 将 Go 返回的 C 字符串转换为 Rust 字符串
    let output = unsafe { CStr::from_ptr(result.output).to_string_lossy().into_owned() };
    let error = unsafe { CStr::from_ptr(result.error).to_string_lossy().into_owned() };

    // 释放 Go 分配的 C 字符串内存，防止内存泄漏
    unsafe {
        FreeResultString(result.output);
        FreeResultString(result.error);
    }

    // 根据 Go 返回的错误信息判断结果
    if !error.is_empty() {
        Err(TemplateError(error))
    } else {
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize}; // 导入 Serialize trait 和 Deserialize (用于测试)
    use serde_json::json; // 导入 serde_json 的 json! 宏

    // 定义一个用于测试的结构体
    #[derive(Serialize, Deserialize)]
    struct User {
        name: String,
        age: u8,
        is_admin: bool,
    }

    #[test]
    fn test_render_simple_template_escaped() {
        let template = "Hello, {{.Name}}!";
        let data = json!({"Name": "World"}); // 使用 json! 宏创建 serde_json::Value
        let result = render_template(template, &data, true).unwrap(); // 默认转义
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_render_template_with_if_escaped() {
        let template = "{{if .Admin}}Admin User{{else}}Regular User{{end}}";
        let data_admin = json!({"Admin": true});
        let data_user = json!({"Admin": false});

        assert_eq!(
            render_template(template, &data_admin, true).unwrap(),
            "Admin User"
        );
        assert_eq!(
            render_template(template, &data_user, true).unwrap(),
            "Regular User"
        );
    }

    #[test]
    fn test_render_template_with_range_escaped() {
        let template = "Items:\n{{range .Items}}- {{.}}\n{{end}}";
        let data = json!({"Items": ["Apple", "Banana", "Cherry"]});
        let expected = "Items:\n- Apple\n- Banana\n- Cherry\n";
        assert_eq!(render_template(template, &data, true).unwrap(), expected);
    }

    #[test]
    fn test_error_handling_invalid_template_escaped() {
        let template = "Invalid {{.Template"; // 语法错误
        let data = json!({});
        let err = render_template(template, &data, true).expect_err("Should return an error");
        assert!(err.to_string().contains("Failed to parse HTML template"));
    }

    #[test]
    fn test_error_handling_invalid_json_serialization() {
        // 模拟一个无法被序列化为有效 JSON 的数据（虽然 serde 很少会这样）
        // 这里实际上是测试 serde_json::to_string 的错误
        #[derive(Debug)]
        struct NonSerializable;
        impl serde::Serialize for NonSerializable {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom(
                    "failed to serialize intentionally",
                ))
            }
        }
        let template = "Hello, {{.Name}}!";
        let data = NonSerializable;
        let err = render_template(template, &data, true).expect_err("Should return an error");
        assert!(err.to_string().contains("Failed to serialize data to JSON"));
    }

    #[test]
    fn test_empty_template_and_data_escaped() {
        let template = "";
        let data = json!({});
        let result = render_template(template, &data, true).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_template_with_no_data_needed_escaped() {
        let template = "This is a static string.";
        let data = json!({});
        let result = render_template(template, &data, true).unwrap();
        assert_eq!(result, "This is a static string.");
    }

    // --- 新增测试用例：不转义 ---

    #[test]
    fn test_render_template_no_escape_html() {
        let template = "<h1>{{.Title}}</h1><p>{{.Content}}</p>";
        let data = json!({"Title": "Test", "Content": "<script>alert('xss')</script>"});
        let result = render_template(template, &data, false).unwrap(); // 不转义
        assert_eq!(result, "<h1>Test</h1><p><script>alert('xss')</script></p>");
    }

    #[test]
    fn test_render_template_with_escape_html() {
        let template = "<h1>{{.Title}}</h1><p>{{.Content}}</p>";
        let data = json!({"Title": "Test", "Content": "<script>alert('xss')</script>"});
        let result = render_template(template, &data, true).unwrap(); // 转义
        assert_eq!(
            result,
            "<h1>Test</h1><p>&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;</p>"
        );
    }

    #[test]
    fn test_error_handling_invalid_template_no_escape() {
        let template = "Invalid {{.Template"; // 语法错误
        let data = json!({});
        let err = render_template(template, &data, false).expect_err("Should return an error");
        assert!(err.to_string().contains("Failed to parse Text template"));
    }

    // --- 使用自定义结构体的测试 ---
    #[test]
    fn test_render_with_struct_data() {
        let template = "User: {{.name}}, Age: {{.age}}, Admin: {{.is_admin}}";
        let user_data = User {
            name: "Alice".to_string(),
            age: 30,
            is_admin: true,
        };
        let result = render_template(template, &user_data, true).unwrap();
        assert_eq!(result, "User: Alice, Age: 30, Admin: true");
    }

    #[test]
    fn test_render_with_struct_data_and_html_content() {
        #[derive(Serialize)]
        struct Product {
            name: String,
            description: String,
        }

        let template = "<h2>{{.name}}</h2><p>{{.description}}</p>";
        let product = Product {
            name: "Shiny Widget".to_string(),
            description: "<p>This is a <strong>great</strong> product!</p>".to_string(),
        };

        // 转义版本
        let result_escaped = render_template(template, &product, true).unwrap();
        assert_eq!(
            result_escaped,
            "<h2>Shiny Widget</h2><p>&lt;p&gt;This is a &lt;strong&gt;great&lt;/strong&gt; product!&lt;/p&gt;</p>"
        );

        // 不转义版本
        let result_unescaped = render_template(template, &product, false).unwrap();
        assert_eq!(
            result_unescaped,
            "<h2>Shiny Widget</h2><p><p>This is a <strong>great</strong> product!</p></p>"
        );
    }
}
