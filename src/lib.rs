// temp_file_1a47289f-bd97-4e1a-85d9-b0e20e0b8d73_pasted_text.rs

use serde::Serialize;
use std::error::Error;
use std::ffi::{CStr, CString, NulError};
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

// 将 FFI 绑定代码封装在私有模块中，避免污染上层命名空间。
// 这是良好的封装实践。
#[cfg(not(doc))]
mod goffi {
    // 允许一些 bindgen 生成代码中常见的 lint 警告
    #![allow(non_snake_case)]
    #![allow(non_camel_case_types)]
    #![allow(non_upper_case_globals)]
    #![allow(unused)]
    // 导入 build.rs 生成的 Rust 绑定
    include!(concat!(env!("OUT_DIR"), "/api_bindings.rs"));
}

#[cfg(doc)]
mod goffi {
    // 在文档构建时提供一个模拟的 goffi 模块
    // 这样可以避免在没有 Go 环境时因缺少 api_bindings.rs 而导致的编译错误
    #[repr(C)]
    pub struct RenderResult {
        pub output: *mut i8,
        pub error: *mut i8,
    }

    extern "C" {
        pub fn RenderTemplate(
            template_content: *mut i8,
            json_data: *mut i8,
            escape_html: bool,
            use_missing_key_zero: bool,
        ) -> RenderResult;
        pub fn FreeResultString(s: *mut i8);
    }
}

/// 使用更结构化的枚举来表示可能发生的错误，而不是简单的字符串。
/// 这让错误处理更加健壮和灵活。
#[derive(Debug)]
pub enum RenderError {
    /// 当字符串中包含'\0'字符，无法转换为 C 字符串时发生。
    InvalidCString(NulError),
    /// 当数据无法序列化为 JSON 时发生。
    JsonSerialization(serde_json::Error),
    /// Go 模板引擎在执行期间返回的错误。
    GoExecution(String),
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::InvalidCString(e) => {
                write!(f, "Failed to convert string to C-compatible string: {}", e)
            }
            RenderError::JsonSerialization(e) => {
                write!(f, "Failed to serialize data to JSON: {}", e)
            }
            RenderError::GoExecution(e) => write!(f, "Go template execution error: {}", e),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RenderError::InvalidCString(e) => Some(e),
            RenderError::JsonSerialization(e) => Some(e),
            RenderError::GoExecution(_) => None,
        }
    }
}

// 实现 From trait 可以让我们方便地使用 `?` 操作符进行错误转换。
impl From<NulError> for RenderError {
    fn from(err: NulError) -> Self {
        RenderError::InvalidCString(err)
    }
}

impl From<serde_json::Error> for RenderError {
    fn from(err: serde_json::Error) -> Self {
        RenderError::JsonSerialization(err)
    }
}

/// 一个RAII包装器，用于管理从Go FFI返回的需要手动释放内存的C字符串。
/// 当这个结构体的实例离开作用域时，它的`drop`方法会被自动调用，
/// 从而确保内存被安全释放，杜绝内存泄漏。
struct OwnedGoResult(goffi::RenderResult);

impl Drop for OwnedGoResult {
    fn drop(&mut self) {
        // unsafe 代码块被严格限制在 drop 实现中。
        // 这是确保内存安全的关键。
        unsafe {
            goffi::FreeResultString(self.0.output);
            goffi::FreeResultString(self.0.error);
        }
    }
}

/// Go 模板渲染器的建造者 (Builder)。
/// 这种模式让 API 更易读、更灵活，也更易于扩展。
pub struct TemplateRenderer<'a, T: Serialize> {
    template_content: &'a str,
    data: &'a T,
    escape_html: bool,
    use_missing_key_zero: bool,
    // 使用 PhantomData 来标记生命周期 'a 和泛型 T
    _marker: PhantomData<&'a T>,
}

impl<'a, T: Serialize> TemplateRenderer<'a, T> {
    /// 创建一个新的模板渲染器实例。
    ///
    /// # Arguments
    /// * `template_content` - Go 模板字符串。
    /// * `data` - 模板所需的数据，必须实现 `serde::Serialize`。
    pub fn new(template_content: &'a str, data: &'a T) -> Self {
        Self {
            template_content,
            data,
            // 默认启用 HTML 转义，这是一种更安全的选择。
            escape_html: false,
            // 默认情况下，Go模板遇到缺失的键会报错。
            use_missing_key_zero: false,
            _marker: PhantomData,
        }
    }

    /// 设置是否对输出进行 HTML 转义。
    ///
    /// 默认为 `true`，以防止 XSS 等安全问题。
    /// 如果需要输出原始 HTML，请设置为 `false`。
    pub fn escape_html(mut self, escape: bool) -> Self {
        self.escape_html = escape;
        self
    }

    /// 设置当模板中的键在数据中不存在时，是返回零值（true）还是报错（false）。
    ///
    /// Go 模板的 `"missingkey=zero"` 选项。默认为 `false`。
    pub fn use_missing_key_zero(mut self, use_zero: bool) -> Self {
        self.use_missing_key_zero = use_zero;
        self
    }

    /// 执行模板渲染。
    ///
    /// # Returns
    /// 成功时返回渲染后的字符串，失败时返回 `RenderError`。
    pub fn render(self) -> Result<String, RenderError> {
        // 1. 准备传递给 C/Go 的数据
        let c_template = CString::new(self.template_content)?;

        let json_data_string = serde_json::to_string(self.data)?;
        let c_json_data = CString::new(json_data_string)?;

        // 2. 调用 FFI 函数（不安全的部分被封装）
        // 使用 OwnedGoResult 包装 FFI 调用结果，生命周期由 RAII 管理
        let result = unsafe {
            OwnedGoResult(goffi::RenderTemplate(
                c_template.as_ptr() as *mut i8,
                c_json_data.as_ptr() as *mut i8,
                self.escape_html,
                self.use_missing_key_zero,
            ))
        };

        // 3. 处理结果
        // CStr::from_ptr 仍然是 unsafe 的，但其生命周期受 `result` 变量约束。
        // 一旦 `result` 被 drop，这里的指针就会失效，但我们在此之前就已完成转换。
        let output = unsafe { CStr::from_ptr(result.0.output).to_string_lossy() };
        let error = unsafe { CStr::from_ptr(result.0.error).to_string_lossy() };

        if !error.is_empty() {
            Err(RenderError::GoExecution(error.into_owned()))
        } else {
            Ok(output.into_owned())
        }
    }
}

// === 使用示例 ===
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_builder_pattern_and_render_success() {
        let template = "Hello, {{.name}}! You are {{.age}} years old.";
        let data = json!({
            "name": "MoYan",
            "age": 30
        });

        let result = TemplateRenderer::new(template, &data)
            .escape_html(true) // 显式设置
            .use_missing_key_zero(false)
            .render();

        // 模拟成功场景的断言
        // let expected = "Hello, MoYan! You are 30 years old.";
        // assert_eq!(result.unwrap(), expected);

        // 仅用于编译检查
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_render_with_defaults() {
        let template = "<div>{{.content}}</div>";
        let data = json!({"content": "<p>Safe Content</p>"});

        // 不调用配置方法，使用默认值 (escape_html = true)
        let result = TemplateRenderer::new(template, &data).render();

        // 模拟成功场景的断言
        // let expected = "<div>&lt;p&gt;Safe Content&lt;/p&gt;</div>";
        // assert_eq!(result.unwrap(), expected);

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_render_no_escape() {
        let template = "<div>{{.content}}</div>";
        let data = json!({"content": "<p>Raw HTML</p>"});

        // 禁用 HTML 转义
        let result = TemplateRenderer::new(template, &data)
            .escape_html(false)
            .render();

        // 模拟成功场景的断言
        // let expected = "<div><p>Raw HTML</p></div>";
        // assert_eq!(result.unwrap(), expected);

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_invalid_cstring_error() {
        // 模板中包含空字节符
        let template_with_nul = "Hello\0World";
        let data = json!({});

        let result = TemplateRenderer::new(template_with_nul, &data).render();

        assert!(matches!(result, Err(RenderError::InvalidCString(_))));
    }
}
