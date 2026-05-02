"""M-Note (comparison note) renderer."""
import textwrap

from core import today_iso


def render_mnote(title: str, a: str, b: str, c: str) -> str:
    today = today_iso()
    md = f"""\
type: comparison
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

"""
    return textwrap.dedent(md).strip() + "\n"
