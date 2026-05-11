//! M-Note (comparison note) renderer.

use chrono::Utc;

/// Render an M-Note (comparison note) markdown document.
pub fn render_mnote(title: &str, a: &str, b: &str, c: &str) -> String {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    format!(
        r#"type: comparison
status: evolving
----------------

# {title}

## 对比维度

| 维度   | A | B | C |
| ---- | - | - | - |
| 核心思想 |   |   |   |
| 成本结构 |   |   |   |
| 性能   |   |   |   |
| 扩展性  |   |   |   |
| 适用场景 |   |   |   |

---

## 当前 A/B/C

- A: {a}
- B: {b}
- C: {c}

---

## 结构性差异

---

## 成本演进分析

---

## 演进方向

---

## 当前判断

---

## View Evolution Log

* {today}

  * 旧观点：
  * 新证据：
  * 更新结论：

"#
    )
}
