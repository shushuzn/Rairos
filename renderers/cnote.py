"""C-Note (concept note) renderer."""
import textwrap


def render_cnote(concept: str) -> str:
    escaped = concept.replace("#", "\\#")
    md = f"""\
type: concept
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
"""
    return textwrap.dedent(md).strip() + "\n"
