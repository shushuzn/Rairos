"""Chart Knowledge Graph - Extract figures/tables from PDFs and store in KG.

Usage:
    from pdf.chart_kg import ChartKGExtractor
    extractor = ChartKGExtractor(kg_manager)
    results = extractor.extract_and_index(pdf_path, paper_uid, paper_title)
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional, Tuple

from pdf.visual import VisualExtractor, VisualContent, ExtractedFigure, TableAsMarkdown
from llm.client import call_llm_chat_completions

logger = logging.getLogger(__name__)


@dataclass
class FigureNode:
    """A figure node for the knowledge graph."""

    entity_id: str  # format: "{paper_uid}_fig_{index}"
    label: str  # human-readable: "Figure 1: Training loss curve"
    description: str  # LLM-generated description
    page: int
    image_path: Optional[str] = None
    caption: str = ""


@dataclass
class TableNode:
    """A table node for the knowledge graph."""

    entity_id: str  # format: "{paper_uid}_table_{index}"
    label: str  # human-readable: "Table 2: Main results"
    description: str  # LLM-generated description
    markdown: str  # table content in markdown
    page: int
    caption: str = ""


class ChartKGExtractor:
    """Extract figures/tables and index them into the knowledge graph."""

    FIGURE_SYSTEM_PROMPT = """你是一个严谨的AI研究助手，擅长解读论文中的图表。

任务：为一篇论文的Figure生成简洁的描述。

要求：
1. 用中文描述图表展示的核心内容（1-3句话）
2. 指出关键发现或趋势
3. 如果是性能对比图，列出具体数字
4. 不要臆测，描述文中明确展示的内容

输出格式：纯文本描述，不超过200字。"""

    TABLE_SYSTEM_PROMPT = """你是一个严谨的AI研究助手，擅长解读论文中的表格。

任务：为一篇论文的Table生成简洁的描述。

要求：
1. 用中文描述表格的核心内容（1-3句话）
2. 列出关键数据（最好/最差、具体数值）
3. 指出表格试图回答什么问题
4. 不要臆测，描述文中明确展示的内容

输出格式：纯文本描述，不超过200字。"""

    def __init__(self, kg_manager, llm_model: str = "qwen3.5-plus"):
        """
        Args:
            kg_manager: KGManager instance for storing nodes
            llm_model: LLM model for generating descriptions
        """
        self.kg = kg_manager
        self.llm_model = llm_model
        self._visual_extractor = VisualExtractor()

    def extract_and_index(
        self,
        pdf_path: str,
        paper_uid: str,
        paper_title: str,
    ) -> Tuple[List[FigureNode], List[TableNode]]:
        """
        Extract figures/tables from PDF and index into KG.

        Args:
            pdf_path: Path to PDF file
            paper_uid: Unique paper identifier
            paper_title: Paper title for context

        Returns:
            (figure_nodes, table_nodes) lists
        """
        visual = self._visual_extractor.extract_visual_content(pdf_path, paper_uid)

        figure_nodes = self._process_figures(visual.figures, paper_uid, paper_title)
        table_nodes = self._process_tables(visual.tables_markdown, paper_uid, paper_title)

        # Store in KG
        for fn in figure_nodes:
            self._index_figure_node(fn, paper_uid)
        for tn in table_nodes:
            self._index_table_node(tn, paper_uid)

        return figure_nodes, table_nodes

    def _process_figures(
        self,
        figures: List[ExtractedFigure],
        paper_uid: str,
        paper_title: str,
    ) -> List[FigureNode]:
        """Process extracted figures with LLM descriptions."""
        nodes = []
        for i, fig in enumerate(figures):
            entity_id = f"{paper_uid}_fig_{i + 1}"
            label = fig.caption if fig.caption else f"Figure {i + 1}"

            # Generate LLM description
            description = self._describe_figure(fig, paper_title)

            nodes.append(
                FigureNode(
                    entity_id=entity_id,
                    label=label,
                    description=description,
                    page=fig.page,
                    image_path=fig.image_path,
                    caption=fig.caption,
                )
            )
        return nodes

    def _process_tables(
        self,
        tables: List[TableAsMarkdown],
        paper_uid: str,
        paper_title: str,
    ) -> List[TableNode]:
        """Process extracted tables with LLM descriptions."""
        nodes = []
        for i, tbl in enumerate(tables):
            entity_id = f"{paper_uid}_table_{i + 1}"
            label = tbl.caption if tbl.caption else f"Table {i + 1}"

            # Generate LLM description
            description = self._describe_table(tbl, paper_title)

            nodes.append(
                TableNode(
                    entity_id=entity_id,
                    label=label,
                    description=description,
                    markdown=tbl.markdown,
                    page=tbl.page,
                    caption=tbl.caption,
                )
            )
        return nodes

    def _describe_figure(self, fig: ExtractedFigure, paper_title: str) -> str:
        """Use LLM to generate figure description."""
        context = f"论文标题: {paper_title}\n"
        context += f"Figure位置: 第{fig.page + 1}页\n"
        if fig.caption:
            context += f"Figure标题: {fig.caption}\n"

        try:
            response = call_llm_chat_completions(
                model=self.llm_model,
                system=self.FIGURE_SYSTEM_PROMPT,
                user=context,
                temperature=0.3,
            )
            return response.content if hasattr(response, "content") else str(response)
        except Exception as e:
            logger.warning(f"LLM description failed: {e}")
            return fig.caption or "无法生成描述"

    def _describe_table(self, tbl: TableAsMarkdown, paper_title: str) -> str:
        """Use LLM to generate table description."""
        context = f"论文标题: {paper_title}\n"
        context += f"Table位置: 第{tbl.page + 1}页\n"
        if tbl.caption:
            context += f"Table标题: {tbl.caption}\n"
        context += f"Table内容:\n{tbl.markdown[:1000]}\n"

        try:
            response = call_llm_chat_completions(
                model=self.llm_model,
                system=self.TABLE_SYSTEM_PROMPT,
                user=context,
                temperature=0.3,
            )
            return response.content if hasattr(response, "content") else str(response)
        except Exception as e:
            logger.warning(f"LLM description failed: {e}")
            return tbl.caption or "无法生成描述"

    def _index_figure_node(self, node: FigureNode, paper_uid: str) -> None:
        """Store figure node in KG and link to paper."""
        # Add figure node
        self.kg.upsert_node(
            node_type="Figure",
            entity_id=node.entity_id,
            label=node.label,
            description=node.description,
            page=node.page,
            image_path=node.image_path or "",
            caption=node.caption,
        )

        # Link to paper node
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        if paper_node:
            fig_node = self.kg.get_node_by_entity("Figure", node.entity_id)
            if paper_node and fig_node:
                self.kg.add_edge(
                    source_id=paper_node["id"],
                    target_id=fig_node["id"],
                    relation_type="has_figure",
                )

    def _index_table_node(self, node: TableNode, paper_uid: str) -> None:
        """Store table node in KG and link to paper."""
        # Add table node
        self.kg.upsert_node(
            node_type="Table",
            entity_id=node.entity_id,
            label=node.label,
            description=node.description,
            page=node.page,
            markdown=node.markdown,
            caption=node.caption,
        )

        # Link to paper node
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        if paper_node:
            tbl_node = self.kg.get_node_by_entity("Table", node.entity_id)
            if paper_node and tbl_node:
                self.kg.add_edge(
                    source_id=paper_node["id"],
                    target_id=tbl_node["id"],
                    relation_type="has_table",
                )

    def get_paper_figures(self, paper_uid: str) -> List[dict]:
        """Get all figures for a paper."""
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        if not paper_node:
            return []

        edges = self.kg.get_edges_by_node(paper_node["id"], direction="out", rel_type="has_figure")
        fig_nodes = []
        for edge in edges:
            target_id = edge["target_id"]
            node = self.kg.get_node(target_id)
            if node and node["type"] == "Figure":
                fig_nodes.append(node)
        return fig_nodes

    def get_paper_tables(self, paper_uid: str) -> List[dict]:
        """Get all tables for a paper."""
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        if not paper_node:
            return []

        edges = self.kg.get_edges_by_node(paper_node["id"], direction="out", rel_type="has_table")
        tbl_nodes = []
        for edge in edges:
            target_id = edge["target_id"]
            node = self.kg.get_node(target_id)
            if node and node["type"] == "Table":
                tbl_nodes.append(node)
        return tbl_nodes

    def query_figure(self, paper_uid: str, figure_label: str) -> Optional[dict]:
        """Query a specific figure by label like 'Figure 3'."""
        figures = self.get_paper_figures(paper_uid)
        for fig in figures:
            if figure_label.lower() in fig["label"].lower():
                return fig
        return None
