package main

/*
#include <stdlib.h> // For C.free
#include <string.h> // For C.strcpy
// 定义一个Go语言中字符串C的表示形式
typedef struct RenderResult {
    char* output;
    char* error;
} RenderResult;

extern RenderResult RenderTemplate(char* templateContent, char* jsonData, _Bool escapeHtml, _Bool useMissingKeyZero);
*/
import "C"
import (
	"bytes"
	"encoding/json"
	"fmt"
	htmltemplate "html/template" // 为 html/template 起别名
	texttemplate "text/template" // 为 text/template 起别名
	"unsafe"        // 用于C语言指针操作
)

type RenderResult struct {
	Output string
	Error  string
}

// renderGoTemplate 是实际的模板渲染逻辑
// 增加了 escapeHtml 和 useMissingKeyZero 参数
func renderGoTemplate(templateContent string, jsonData string, escapeHtml bool, useMissingKeyZero bool) RenderResult {
	var data map[string]interface{}
	err := json.Unmarshal([]byte(jsonData), &data)
	if err != nil {
		return RenderResult{
			Error: fmt.Sprintf("Failed to unmarshal JSON data: %v", err),
		}
	}

	var buf bytes.Buffer

	tmplOptions := "missingkey="
	if useMissingKeyZero {
		tmplOptions += "zero"
	} else {
		tmplOptions += "default"
	}

	if escapeHtml {
		// 使用 html/template 确保安全性，防止 XSS
		// 根据 tmplOptions 创建模板
		tmpl := htmltemplate.New("goTemplate").Option(tmplOptions)
		tmpl, err = tmpl.Parse(templateContent)
		if err != nil {
			return RenderResult{
				Error: fmt.Sprintf("Failed to parse HTML template: %v", err),
			}
		}
		err = tmpl.Execute(&buf, data)
		if err != nil {
			return RenderResult{
				Error: fmt.Sprintf("Failed to execute HTML template: %v", err),
			}
		}
	} else {
		// 使用 text/template 渲染，不进行 HTML 转义
		// 根据 tmplOptions 创建模板
		tmpl := texttemplate.New("goTemplate").Option(tmplOptions)
		tmpl, err = tmpl.Parse(templateContent);
		if err != nil {
			return RenderResult{
				Error: fmt.Sprintf("Failed to parse Text template: %v", err),
			}
		}
		err = tmpl.Execute(&buf, data)
		if err != nil {
			return RenderResult{
				Error: fmt.Sprintf("Failed to execute Text template: %v", err),
			}
		}
	}

	return RenderResult{
		Output: buf.String(),
		Error:  "",
	}
}

// RenderTemplate 是暴露给 C 的函数。
// 增加了 cEscapeHtml 和 cUseMissingKeyZero 参数。
//export RenderTemplate
func RenderTemplate(cTemplateContent *C.char, cJsonData *C.char, cEscapeHtml C._Bool, cUseMissingKeyZero C._Bool) C.RenderResult {
	templateContent := C.GoString(cTemplateContent)
	jsonData := C.GoString(cJsonData)
	escapeHtml := bool(cEscapeHtml)
	useMissingKeyZero := bool(cUseMissingKeyZero) // 将 C._Bool 转换为 Go bool

	result := renderGoTemplate(templateContent, jsonData, escapeHtml, useMissingKeyZero)

	cOutput := C.CString(result.Output)
	cError := C.CString(result.Error)

	return C.RenderResult{
		output: cOutput,
		error:  cError,
	}
}

// FreeResultString 是一个辅助函数，用于释放 C 字符串内存，防止内存泄漏。
// Rust 端在接收到字符串后，需要调用此函数来释放 Go 分配的内存。
//export FreeResultString
func FreeResultString(cStr *C.char) {
	C.free(unsafe.Pointer(cStr))
}

func main() {
	// main 函数必须存在，但在这里是空的，因为我们是编译为 C 共享库。
}

