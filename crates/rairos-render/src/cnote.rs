//! C-Note (concept note) renderer.

/// Render a C-Note markdown document from a concept name.
pub fn render_cnote(concept: &str) -> String {
    let escaped = concept.replace('#', "\\#");
    format!(
        r#"type: concept
status: evergreen
-----------------

# {escaped}

## 核心定义
## 产生背景
## 技术本质
## 常见实现路径
## 优势
## 局限
## 与其他思想的关系
## 代表论文
## 演化时间线
## 未来趋势
## 关联笔记
"#
    )
}
