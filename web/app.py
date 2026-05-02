"""Streamlit Web UI for AI Research OS.

Run: streamlit run web/app.py
"""
from __future__ import annotations

import sys
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

import streamlit as st

st.set_page_config(page_title="AI Research OS", layout="wide", page_icon="🧠")

st.title("🧠 AI Research OS — Web UI")


# ─── DB helpers ──────────────────────────────────────────────────────

def _get_db():
    if "db" not in st.session_state:
        from db.database import Database
        db = Database()
        db.init()
        st.session_state["db"] = db
    return st.session_state["db"]


def init_kg():
    if "kg" not in st.session_state:
        from kg.manager import KGManager
        st.session_state["kg"] = KGManager()


def init_scoring():
    if "scoring" not in st.session_state:
        from scoring.momentum import ResearchMomentum
        st.session_state["scoring"] = ResearchMomentum()


# ─── Sidebar navigation ────────────────────────────────────────────

st.sidebar.title("Navigation")
page = st.sidebar.radio("Go to", [
    "📊 Dashboard",
    "📚 Papers",
    "📄 Paper Detail",
    "📥 Import",
    "💬 Chat",
    "📈 Momentum Scores",
    "📊 KG Stats",
    "🔗 KG Graph",
    "📋 Experiment Tables",
    "📉 Trends",
    "⚙️ Process",
    # ── Innovation pages ──
    "🧬 Gene Pool",
    "🎯 Gap Detection",
    "🔄 InsightEvolution",
    "🔍 MCP Research",
    "🚀 Autopilot Watch",
    "🎯 Hypothesis Lab",
    "📚 Lit Review",
    "🧠 Research Memory",
    "🎯 Auto Reviewer",
    "🗺️ Route Planner",
    "📋 Research Briefing",
])

# ─── Dashboard ─────────────────────────────────────────────────────

if page == "📊 Dashboard":
    db = _get_db()
    stats = db.get_stats()

    col1, col2, col3, col4 = st.columns(4)
    col1.metric("Total Papers", stats["total_papers"])
    col2.metric("Parsed", stats["by_status"].get("parsed", 0))
    col3.metric("Queued", stats["queue_queued"] + stats["queue_running"])
    col4.metric("Cache Entries", stats["cache_entries"])

    st.subheader("Papers by Source")
    if stats["by_source"]:
        st.bar_chart(stats["by_source"])
    else:
        st.info("No papers yet. Import some papers to get started.")

    st.subheader("Papers by Status")
    if stats["by_status"]:
        st.bar_chart(stats["by_status"])

    st.subheader("Recent Papers")
    recent, _ = db.list_papers(limit=10, sort_by="added_at", sort_order="desc")
    if recent:
        for p in recent:
            authors = ", ".join(p.authors[:3]) if p.authors else ""
            st.write(f"**{p.title[:80]}** — {authors} ({p.published[:4] if p.published else '?'})")
    else:
        st.info("No papers yet.")


# ─── Papers (List + Search) ────────────────────────────────────────

elif page == "📚 Papers":
    db = _get_db()

    with st.sidebar:
        st.subheader("Filters")
        source = st.selectbox("Source", ["All", "arxiv", "doi"], index=0)
        sort_by = st.selectbox("Sort by", ["added_at", "published", "title"], index=0)
        sort_order = st.selectbox("Order", ["desc", "asc"], index=0)
        tag_filter = st.text_input("Filter by tag", "")

    query = st.text_input("Search papers (FTS5 + BM25 ranking)", "")

    # Pagination
    if "page_offset" not in st.session_state:
        st.session_state["page_offset"] = 0
    page_size = 20

    col_prev, col_info, col_next = st.columns([1, 2, 1])
    with col_prev:
        if st.button("← Previous") and st.session_state["page_offset"] > 0:
            st.session_state["page_offset"] = max(0, st.session_state["page_offset"] - page_size)
    with col_next:
        if st.button("Next →"):
            st.session_state["page_offset"] += page_size

    offset = st.session_state["page_offset"]

    if query.strip():
        results, total = db.search_papers(
            query=query.strip(),
            limit=page_size,
            offset=offset,
            source=source if source != "All" else None,
        )
        with col_info:
            st.write(f"**{total}** results for **{query}** (page {offset // page_size + 1})")
        for r in results:
            with st.expander(f"**{r.title[:80]}** [{r.score:.2f}]"):
                st.write(f"**Authors:** {', '.join(r.authors[:5]) if r.authors else 'N/A'}")
                st.write(f"**Year:** {r.published[:4] if r.published else 'N/A'} | **Source:** {r.source} | **Category:** {r.primary_category}")
                if r.snippet:
                    st.write(f"**Snippet:** ...{r.snippet}...")
                if r.abs_url:
                    st.markdown(f"[View Paper]({r.abs_url})")
    else:
        papers, total = db.list_papers(
            limit=page_size,
            offset=offset,
            source=source if source != "All" else None,
            sort_by=sort_by,
            sort_order=sort_order,
        )
        with col_info:
            st.write(f"**{total}** papers (page {offset // page_size + 1})")
        for p in papers:
            authors = ", ".join(p.authors[:3]) if p.authors else ""
            status_icon = "✅" if p.parse_status == "parsed" else "⏳" if p.parse_status == "pending" else "❌"
            with st.expander(f"{status_icon} **{p.title[:80]}**"):
                st.write(f"**Authors:** {authors}")
                st.write(f"**Year:** {p.published[:4] if p.published else 'N/A'} | **Source:** {p.source} | **Status:** {p.parse_status}")
                if p.abstract:
                    st.write(f"**Abstract:** {p.abstract[:300]}...")
                if p.abs_url:
                    st.markdown(f"[View Paper]({p.abs_url})")


# ─── Paper Detail ───────────────────────────────────────────────────

elif page == "📄 Paper Detail":
    db = _get_db()

    paper_id = st.text_input("Paper ID (e.g. 2601.00155)", value=st.query_params.get("paper_id", ""))

    if paper_id:
        paper = db.get_paper(paper_id)
        if paper is None:
            st.error(f"Paper '{paper_id}' not found.")
        else:
            st.header(paper.title)
            authors = ", ".join(paper.authors) if paper.authors else "N/A"
            st.write(f"**Authors:** {authors}")
            st.write(f"**Published:** {paper.published} | **Source:** {paper.source} | **Category:** {paper.primary_category}")
            st.write(f"**Parse Status:** {paper.parse_status}")

            if paper.abs_url:
                st.markdown(f"[View on {paper.source}]({paper.abs_url})")
            if paper.pdf_url:
                st.markdown(f"[Download PDF]({paper.pdf_url})")

            # Tags
            tags = db.get_tags(paper_id)
            if tags:
                st.subheader("Tags")
                st.write(", ".join(f"`{t}`" for t in tags))

            # Abstract
            if paper.abstract:
                st.subheader("Abstract")
                st.write(paper.abstract)

            # P-Note
            st.subheader("Paper Note (P-Note)")
            pnote_path = Path("notes") / paper_id / "P-Note.md"
            if pnote_path.exists():
                st.markdown(pnote_path.read_text(encoding="utf-8"))
            else:
                st.info("P-Note not found. Run postprocessing to generate notes.")

            # C-Note
            st.subheader("Concept Note (C-Note)")
            cnote_path = Path("notes") / paper_id / "C-Note.md"
            if cnote_path.exists():
                st.markdown(cnote_path.read_text(encoding="utf-8"))
            else:
                st.info("C-Note not found.")

            # Text stats
            if paper.word_count > 0:
                st.subheader("Document Stats")
                c1, c2, c3, c4 = st.columns(4)
                c1.metric("Words", f"{paper.word_count:,}")
                c2.metric("Pages", paper.page_count)
                c3.metric("Tables", paper.table_count)
                c4.metric("Figures", paper.figure_count)
    else:
        st.info("Enter a paper ID above to view its details.")


# ─── Import ─────────────────────────────────────────────────────────

elif page == "📥 Import":
    db = _get_db()

    st.subheader("Import Papers")
    st.write("Import papers by arXiv ID, arXiv URL, or DOI.")

    with st.form("import_form"):
        paper_input = st.text_area(
            "Paper IDs (one per line)",
            placeholder="2601.00155\n10.1038/nature12373\nhttps://arxiv.org/abs/2601.00155",
            height=100,
        )
        tags_input = st.text_input("Tags (comma-separated)", placeholder="LLM, Agent, RAG")
        submitted = st.form_submit_button("Import")

    if submitted and paper_input.strip():
        from parsers.input_detection import is_probably_doi, normalize_doi
        from parsers.arxiv import fetch_arxiv_metadata
        from parsers.crossref import fetch_crossref_metadata

        paper_ids = [line.strip() for line in paper_input.strip().splitlines() if line.strip()]
        tags = [t.strip() for t in tags_input.split(",") if t.strip()] if tags_input else []

        progress = st.progress(0)
        status = st.empty()

        for i, raw_id in enumerate(paper_ids):
            # Normalize: extract arXiv ID from URL
            if "arxiv.org" in raw_id:
                parts = raw_id.split("/")
                raw_id = parts[-1].split("v")[0]  # strip version

            status.text(f"Importing {raw_id}...")
            progress.progress((i + 1) / len(paper_ids))

            try:
                if is_probably_doi(raw_id):
                    doi = normalize_doi(raw_id)
                    paper_obj, _ = fetch_crossref_metadata(doi)
                    source = "doi"
                    paper_id = doi if doi else ""
                else:
                    paper_obj = fetch_arxiv_metadata(raw_id)
                    source = "arxiv"
                    paper_id = raw_id

                db.upsert_paper(
                    paper_id=paper_id,
                    source=source,
                    title=paper_obj.title or "",
                    authors=paper_obj.authors or [],
                    abstract=paper_obj.abstract or "",
                    published=paper_obj.published or "",
                    abs_url=paper_obj.abs_url or "",
                    pdf_url=paper_obj.pdf_url or "",
                    primary_category=paper_obj.primary_category or "",
                    doi=paper_obj.doi or "",
                )

                # Add tags
                for tag in tags:
                    db.add_tag(paper_id, tag)

                st.success(f"✅ Imported: **{paper_obj.title[:60]}** ({paper_id})")
            except Exception as e:
                st.error(f"❌ Failed to import {raw_id}: {e}")

        progress.empty()
        status.empty()
        st.balloons()


# ─── Chat ───────────────────────────────────────────────────────────

elif page == "💬 Chat":
    db = _get_db()

    # Session management + AI config in sidebar
    with st.sidebar:
        st.subheader("AI Settings")
        api_key = st.text_input("API Key", value=st.session_state.get("ai_api_key", ""), type="password", help="OpenAI or Anthropic API key")
        base_url = st.text_input("Base URL", value=st.session_state.get("ai_base_url", "https://api.openai.com/v1"), help="OpenAI-compatible API base URL")
        model = st.text_input("Model", value=st.session_state.get("ai_model", "qwen3.5-plus"), help="Model name (e.g. gpt-4o, qwen3.5-plus, claude-sonnet-4-20250514)")
        if st.button("Save Settings"):
            st.session_state["ai_api_key"] = api_key
            st.session_state["ai_base_url"] = base_url
            st.session_state["ai_model"] = model
            # Reset ResearchChat so it picks up new settings
            st.session_state.pop("research_chat", None)
            st.success("Settings saved!")

        st.divider()
        st.subheader("Chat Sessions")
        sessions = db.get_chat_sessions(limit=30)
        if sessions:
            session_options = {s["title"] or s["id"][:8]: s["id"] for s in sessions}
            selected_title = st.selectbox("Select session", list(session_options.keys()))
            selected_session_id = session_options[selected_title]
        else:
            selected_session_id = None

        if st.button("New Session"):
            new_id = str(uuid.uuid4())[:8]
            db.create_chat_session(new_id, f"Chat {new_id}")
            st.session_state["chat_session"] = new_id
            st.rerun()

    # Get active session
    chat_session_id = st.session_state.get("chat_session") or selected_session_id

    if chat_session_id:
        messages = db.get_chat_messages(chat_session_id)

        # Display history
        for msg in messages:
            with st.chat_message(msg["role"]):
                st.write(msg["content"])
                if msg.get("citations"):
                    with st.expander("Citations"):
                        st.json(msg["citations"])

        # Initialize ResearchChat with DB context + user settings
        if "research_chat" not in st.session_state:
            from llm.research_chat import ResearchChat
            init_kg()
            st.session_state["research_chat"] = ResearchChat(
                db=db,
                kg=st.session_state.get("kg"),
                api_key=st.session_state.get("ai_api_key"),
                base_url=st.session_state.get("ai_base_url"),
                model=st.session_state.get("ai_model"),
            )
        rc = st.session_state["research_chat"]

        # Input
        if prompt := st.chat_input("Ask about your papers..."):
            with st.chat_message("user"):
                st.write(prompt)
            db.add_chat_message(chat_session_id, "user", prompt)

            with st.chat_message("assistant"):
                try:
                    response = st.write_stream(rc.chat_stream(prompt))
                except Exception as e:
                    response = (
                        f"AI call failed: {e}\n\n"
                        "Configure your API key in the sidebar under **AI Settings**,\n"
                        "or set environment variables: `OPENAI_API_KEY` or `ANTHROPIC_API_KEY`."
                    )
                    st.write(response)
                db.add_chat_message(chat_session_id, "assistant", response)
    else:
        st.info("Create or select a chat session to start.")


# ─── Momentum Scores ───────────────────────────────────────────────

elif page == "📈 Momentum Scores":
    init_scoring()
    scoring = st.session_state["scoring"]

    st.subheader("Tag Momentum Leaderboard")
    leaderboard = scoring.get_tag_leaderboard()
    if leaderboard:
        tags, scores = zip(*leaderboard)
        st.table({"Tag": tags[:20], "Momentum Score": scores[:20]})
    else:
        st.info("No tags in KG yet. Run `kg rebuild` first.")

    st.subheader("Top Papers")
    top_n = st.slider("Top N", 5, 50, 20)
    top_papers = scoring.get_top_papers(top_n=top_n)
    if top_papers:
        uids, scores = zip(*top_papers)
        st.table({"Paper UID": uids, "Score": scores})
    else:
        st.info("No papers scored yet.")


# ─── KG Stats ──────────────────────────────────────────────────────

elif page == "📊 KG Stats":
    import orjson
    init_kg()
    kg = st.session_state["kg"]

    if st.button("Rebuild KG from DB"):
        from kg.integration import KGIntegration
        integ = KGIntegration(kg)
        integ.rebuild_from_db(_get_db())
        st.success("KG rebuilt!")

    stats = kg.stats()
    st.json(stats)

    if st.button("Export KG as JSON"):
        from kg.queries import KGQueries
        q = KGQueries(kg)
        export = q.export_graph_json()
        st.download_button(
            "Download graph JSON",
            orjson.dumps(export, option=orjson.OPT_INDENT_2),
            file_name="kg_export.json",
            mime="application/json",
        )


# ─── KG Graph ──────────────────────────────────────────────────────

elif page == "🔗 KG Graph":
    init_kg()
    kg = st.session_state["kg"]

    st.subheader("Knowledge Graph Visualizer")
    viz_mode = st.radio("Mode", ["Paper ego graph", "Tag ecosystem", "Full graph"])

    if viz_mode == "Paper ego graph":
        paper_id = st.text_input("Paper UID", "")
        depth = st.slider("Depth", 1, 3, 2)
        if paper_id:
            from viz.pyvis_renderer import KGVizRenderer
            renderer = KGVizRenderer(kg)
            html = renderer.paper_graph(paper_id, depth=depth)
            st.components.v1.html(html, height=600, scrolling=True)

    elif viz_mode == "Tag ecosystem":
        tag = st.text_input("Tag", "")
        if tag:
            from viz.pyvis_renderer import KGVizRenderer
            renderer = KGVizRenderer(kg)
            html = renderer.tag_graph(tag)
            st.components.v1.html(html, height=600, scrolling=True)

    else:
        max_nodes = st.slider("Max nodes", 50, 500, 200)
        from viz.pyvis_renderer import KGVizRenderer
        renderer = KGVizRenderer(kg)
        html = renderer.full_graph(max_nodes=max_nodes)
        st.components.v1.html(html, height=800, scrolling=True)


# ─── Experiment Tables ─────────────────────────────────────────────

elif page == "📋 Experiment Tables":
    st.subheader("Experiment Table Search")

    metric = st.text_input("Metric (e.g. Accuracy, BLEU)", "")
    dataset = st.text_input("Dataset (e.g. SQuAD, GLUE)", "")
    model = st.text_input("Model", "")
    min_val = st.number_input("Min value", value=0.0, step=0.1)

    if st.button("Search"):
        from extable.storage import ExperimentDB
        db = ExperimentDB()
        results = db.search_tables(
            metric=metric or None,
            dataset=dataset or None,
            model=model or None,
            min_value=min_val if min_val > 0 else None,
        )
        st.write(f"**{len(results)} table(s)**")
        for t in results:
            with st.expander(f"Table: {t['caption'][:60]}"):
                st.json(t)

    st.subheader("DB Stats")
    from extable.storage import ExperimentDB
    db = ExperimentDB()
    st.json(db.stats())


# ─── Trends ─────────────────────────────────────────────────────────

elif page == "📉 Trends":
    st.subheader("Trend Forecasting")

    from trends.forecaster import TrendForecaster
    tf = TrendForecaster()

    if st.button("Record Radar Snapshot"):
        tf.record_current_radar()
        st.success("Radar snapshot recorded.")

    st.subheader("Trending Tags (rising)")
    trending = tf.detect_trending()
    if trending:
        tags, slopes = zip(*trending)
        st.table({"Tag": tags[:20], "Slope": slopes[:20]})
    else:
        st.info("No trending data. Record radar snapshots first.")

    st.subheader("Top Predictions (next hot)")
    preds = tf.get_top_predictions(top_k=10)
    if preds:
        for p in preds:
            st.write(f"**{p['tag']}** — predicted: {p['predicted']}, confidence: {p['confidence']}, trend: {p['trend']}")
    else:
        st.info("No prediction data yet.")

    st.subheader("Tag Comparison")
    t1 = st.text_input("Tag A", key="tag_a")
    t2 = st.text_input("Tag B", key="tag_b")
    if t1 and t2:
        comp = tf.compare_tags(t1, t2)
        st.json(comp)


# ─── Process Papers ───────────────────────────────────────────────

elif page == "⚙️ Process":
    db = _get_db()
    stats = db.get_stats()

    st.subheader("Paper Processing Pipeline")

    # Status overview
    col1, col2, col3, col4 = st.columns(4)
    col1.metric("Total Papers", stats["total_papers"])
    col2.metric("Pending", stats["by_status"].get("pending", 0))
    col3.metric("Parsed", stats["by_status"].get("parsed", 0))
    col4.metric("Failed", stats["by_status"].get("failed", 0))

    st.divider()

    # ── Batch Process ────────────────────────────────────────────
    st.subheader("Batch Process Papers")

    with st.expander("Processing Options", expanded=False):
        use_llm = st.checkbox("Use LLM for deep analysis (requires API key)", value=True)
        if use_llm:
            proc_api_key = st.text_input(
                "API Key", value=st.session_state.get("ai_api_key", ""),
                type="password", key="proc_api_key",
            )
            proc_base_url = st.text_input(
                "Base URL", value=st.session_state.get("ai_base_url", "https://api.openai.com/v1"),
                key="proc_base_url",
            )
            proc_model = st.text_input(
                "Model", value=st.session_state.get("ai_model", "qwen3.5-plus"),
                key="proc_model",
            )
        max_pages = st.slider("Max pages to extract", 5, 100, 30)

    if st.button("🚀 Process All Pending Papers", type="primary"):
        # Get all pending papers
        all_papers, _ = db.list_papers(limit=1000)
        pending = [p for p in all_papers if p.parse_status == "pending"]

        if not pending:
            st.info("No pending papers to process.")
        else:
            st.session_state["processing"] = True
            st.session_state["process_results"] = []

            progress = st.progress(0, text="Starting...")
            status_area = st.status("Processing papers...", expanded=True)

            from pdf.extract import download_pdf, extract_pdf_text_hybrid
            from pathlib import Path
            import time

            cache_dir = Path("cache")
            cache_dir.mkdir(exist_ok=True)

            succeeded, failed = 0, 0

            for i, paper in enumerate(pending):
                pid = paper.id
                status_area.write(f"**[{i+1}/{len(pending)}]** {paper.title[:60]}...")

                try:
                    # Step 1: Download PDF
                    pdf_url = getattr(paper, "pdf_url", "") or ""
                    if not pdf_url and pid.startswith("10."):
                        status_area.write("  ⏭️ Skipping DOI paper (no direct PDF URL)")
                        failed += 1
                        continue

                    if pdf_url:
                        pdf_path = cache_dir / f"{pid}.pdf"
                        if not pdf_path.exists():
                            status_area.write("  📥 Downloading PDF...")
                            download_pdf(pdf_url, pdf_path)
                            status_area.write("  ✅ PDF downloaded")
                        else:
                            status_area.write("  📄 PDF already cached")
                    else:
                        status_area.write("  ⚠️ No PDF URL — trying arXiv fallback")
                        # Try arXiv PDF URL
                        if not pid.startswith("10."):
                            pdf_url = f"https://arxiv.org/pdf/{pid}"
                            pdf_path = cache_dir / f"{pid}.pdf"
                            if not pdf_path.exists():
                                download_pdf(pdf_url, pdf_path)
                            status_area.write("  ✅ PDF downloaded from arXiv")
                        else:
                            failed += 1
                            continue

                    # Step 2: Extract text
                    status_area.write("  📝 Extracting text...")
                    extracted_text = extract_pdf_text_hybrid(pdf_path, max_pages=max_pages)
                    word_count = len(extracted_text.split())
                    status_area.write(f"  ✅ Extracted {word_count:,} words")

                    # Step 3: Update DB with PDF path
                    db.upsert_paper(
                        paper_id=pid, source=paper.source,
                        pdf_path=str(pdf_path),
                    )

                    # Step 4: Run pipeline (if LLM enabled)
                    if use_llm and proc_api_key:
                        status_area.write("  🧠 Running deep analysis pipeline...")
                        from llm.postprocess import (
                            ResearchDeepDivePipeline, PostStage, make_llm_config,
                        )
                        from core import Paper as PaperObj
                        from core.basics import slugify_title

                        paper_obj = PaperObj(
                            source=getattr(paper, "source", "arxiv"),
                            uid=pid,
                            title=paper.title,
                            authors=paper.authors or [],
                            abstract=paper.abstract or "",
                            published=paper.published or "",
                            updated="",
                            abs_url=getattr(paper, "abs_url", "") or "",
                            pdf_url=pdf_url,
                            primary_category=getattr(paper, "primary_category", "") or "",
                        )

                        # P-note path
                        year = (paper.published or "")[:4] or str(time.localtime().tm_year)
                        pnote_dir = Path("notes") / pid
                        pnote_dir.mkdir(parents=True, exist_ok=True)
                        pnote_path = pnote_dir / "P-Note.md"

                        pipeline = ResearchDeepDivePipeline(db=db, data_dir=Path("."))
                        result = pipeline.run(
                            paper_id=pid,
                            extracted_text=extracted_text,
                            paper=paper_obj,
                            tags=getattr(paper, "tags", []) or [],
                            pnote_path=pnote_path,
                            llm_config={
                                "api_key": proc_api_key,
                                "base_url": proc_base_url,
                                "model": proc_model,
                            },
                        )
                        if result.stages_completed:
                            status_area.write(f"  ✅ Pipeline: {', '.join(result.stages_completed)}")
                        if result.stages_failed:
                            status_area.write(f"  ⚠️ Issues: {', '.join(result.stages_failed)}")
                    else:
                        # No LLM: just mark as parsed with extracted text
                        status_area.write("  ℹ️ LLM disabled — text extracted only")

                    # Mark as parsed
                    db.update_parse_status(
                        paper_id=pid,
                        status="parsed",
                        plain_text=extracted_text[:50000],
                        word_count=word_count,
                        page_count=0,
                    )
                    succeeded += 1
                    st.session_state["process_results"].append({"id": pid, "status": "ok"})

                except Exception as e:
                    status_area.write(f"  ❌ Error: {e}")
                    db.update_parse_status(pid, status="failed", error=str(e)[:200])
                    failed += 1
                    st.session_state["process_results"].append({"id": pid, "status": "error", "error": str(e)[:100]})

                progress.progress(
                    (i + 1) / len(pending),
                    text=f"Processed {i+1}/{len(pending)}",
                )

            status_area.update(label=f"Done! ✅ {succeeded} succeeded, ❌ {failed} failed", state="complete")
            progress.empty()
            st.session_state["processing"] = False

            if succeeded > 0:
                st.success(f"Processed {succeeded} papers successfully!")
            if failed > 0:
                st.warning(f"{failed} papers failed processing.")

    # Show last results
    if "process_results" in st.session_state and st.session_state["process_results"]:
        results = st.session_state["process_results"]
        ok = [r for r in results if r["status"] == "ok"]
        errs = [r for r in results if r["status"] == "error"]
        if errs:
            with st.expander(f"❌ Failed papers ({len(errs)})"):
                for r in errs:
                    st.write(f"**{r['id']}**: {r.get('error', 'unknown')}")

    st.divider()

    # ── KG Rebuild ───────────────────────────────────────────────
    st.subheader("Knowledge Graph")

    col_a, col_b = st.columns(2)
    with col_a:
        if st.button("🔄 Rebuild KG from DB"):
            init_kg()
            from kg.integration import KGIntegration
            integ = KGIntegration(st.session_state["kg"])
            integ.rebuild_from_db(db)
            st.success("KG rebuilt from database!")

    with col_b:
        if st.button("📊 View KG Stats"):
            init_kg()
            kg_stats = st.session_state["kg"].stats()
            st.json(kg_stats)
# ─── Gene Pool Page ───────────────────────────────────────────────────────

elif page == "🧬 Gene Pool":
    st.header("🧬 Gene Pool — Self-Evolving Success Patterns")

    with st.expander("ℹ️ What is the Gene Pool?", expanded=False):
        st.markdown("""
The **Gene Pool** stores *CapsuleGenes* — encoded success patterns from gaps you accepted.
Every time you accept a gap, the system records:
- **Trigger**: what context (topic, gap type, keywords) led to your action
- **Action**: what gap type you accepted
- **Outcome**: how well it worked (success score from accept/reject ratio)

When detecting new gaps, the system matches them against your Gene Pool to inject a
**success pattern signal** — gaps that match your proven interests rank higher.
        """)

    # Init tracker
    if "evo_tracker" not in st.session_state:
        from llm.insight.tracker import EvolutionTracker
        st.session_state["evo_tracker"] = EvolutionTracker()

    tracker = st.session_state["evo_tracker"]

    # Gene Pool Stats
    stats = tracker.get_gene_pool_stats()
    c1, c2, c3, c4 = st.columns(4)
    c1.metric("Total Capsules", stats["total"])
    c2.metric("Avg Success Score", f"{stats['avg_score']:.3f}")
    c3.metric("Generations", len(stats.get("generations", [0])))
    by_type = stats.get("by_gap_type", {})
    c4.metric("Gap Types", len(by_type))

    st.divider()

    # Gap Type Distribution
    if by_type:
        st.subheader("Capsules by Gap Type")
        types = list(by_type.keys())
        counts = list(by_type.values())
        st.bar_chart({"Gap Type": types, "Count": counts})

    st.divider()

    # List all capsules
    st.subheader("All Capsules")
    capsules = []
    gp_file = tracker._gene_pool_file
    if gp_file.exists():
        import json
        with open(gp_file, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        capsules.append(json.loads(line))
                    except Exception:
                        pass

    if capsules:
        # Sort by success score desc
        capsules.sort(key=lambda x: x.get("outcome_success_score", 0), reverse=True)

        for i, cap in enumerate(capsules):
            score = cap.get("outcome_success_score", 0)
            score_bar = "🟢" if score >= 0.7 else "🟡" if score >= 0.4 else "🔴"
            with st.expander(f"{score_bar} [{cap.get('capsule_id', '?')[:8]}] {cap.get('action_gap_type', '?')} — {cap.get('action_gap_title', '?')[:50]}"):
                col_a, col_b = st.columns(2)
                with col_a:
                    st.write(f"**Trigger Topic:** {cap.get('trigger_topic', '?')}")
                    st.write(f"**Trigger Gap Type:** {cap.get('trigger_gap_type', '?')}")
                    st.write(f"**Trigger Keywords:** {', '.join(cap.get('trigger_keywords', [])[:8])}")
                with col_b:
                    st.write(f"**Action Gap Type:** {cap.get('action_gap_type', '?')}")
                    st.write(f"**Action Gap Title:** {cap.get('action_gap_title', '?')[:60]}")
                    st.write(f"**Success Score:** {score:.3f}")
                    st.write(f"**Feedback Count:** {cap.get('feedback_count', 0)}")
                    st.write(f"**Generation:** {cap.get('evolved_generation', 0)}")
                    st.write(f"**Created:** {cap.get('created_at', '?')}")
    else:
        st.info("No capsules in Gene Pool yet. Accept some gaps to populate it!")

    st.divider()

    # Record a manual accept (for testing)
    st.subheader("Record Feedback")
    with st.expander("Simulate Accept/Reject (for testing)", expanded=False):
        col_t, col_g, col_title = st.columns(3)
        with col_t:
            sim_topic = st.text_input("Topic", value="RLHF", key="sim_topic")
        with col_g:
            sim_gtype = st.selectbox("Gap Type", [
                "method_limitation", "unexplored_application", "contradiction",
                "evaluation_gap", "scalability_issue", "theoretical_gap",
                "dataset_gap", "generalization_gap",
            ], key="sim_gtype")
        with col_title:
            sim_gtitle = st.text_input("Gap Title", value="Reward hacking in RLHF", key="sim_gtitle")

        col_acc, col_rej = st.columns(2)
        with col_acc:
            if st.button("✅ Record Accept", key="sim_accept"):
                tracker.record_gap_accept(
                    topic=sim_topic,
                    gap_type=sim_gtype,
                    gap_title=sim_gtitle,
                )
                st.success(f"Accepted: {sim_gtype} — {sim_gtitle}")
                st.rerun()
        with col_rej:
            if st.button("❌ Record Reject", key="sim_reject"):
                tracker.record_gap_reject(
                    topic=sim_topic,
                    gap_type=sim_gtype,
                    gap_title=sim_gtitle,
                    reason="Not relevant",
                )
                st.warning(f"Rejected: {sim_gtype} — {sim_gtitle}")
                st.rerun()


# ─── Gap Detection Page ─────────────────────────────────────────────────────

elif page == "🎯 Gap Detection":
    st.header("🎯 Gap Detection — Preference-Aware Ranking")

    with st.expander("ℹ️ How does it work?", expanded=False):
        st.markdown("""
**Gap Detection** finds research gaps from your papers and ranks them using a **6-tuple**:

| Dimension | Range | Description |
|-----------|-------|-------------|
| `trend` | 0.0–2.0 | Trending keyword boost from hot topics |
| `gene_pool` | 0.0–1.0 | Success pattern match from your Gene Pool |
| `keyword` | 0.0–3.0 | Matches your top keywords |
| `pref` | -2 to +2 | Liked/disliked gap types |
| `severity` | 1–3 | HIGH/MEDIUM/LOW severity |
| `priority` | int | Evidence strength |

Gaps are sorted by **lexicographic order** on this tuple — trend first, then gene_pool.
The Gene Pool signal is the **strongest differentiator** between similar gaps.
        """)

    # Init tracker and gap analyzer
    if "evo_tracker" not in st.session_state:
        from llm.insight.tracker import EvolutionTracker
        st.session_state["evo_tracker"] = EvolutionTracker()

    if "gap_analyzer_v2" not in st.session_state:
        from llm.gap_analyzer import GapAnalyzerV2
        tracker = st.session_state["evo_tracker"]
        st.session_state["gap_analyzer_v2"] = GapAnalyzerV2(
            evolution_tracker=tracker
        )

    analyzer = st.session_state["gap_analyzer_v2"]
    tracker = st.session_state["evo_tracker"]

    # Topic input
    topic = st.text_input("🎯 Research Topic", value="RLHF", placeholder="e.g., RLHF, RAG, Diffusion Models")
    min_papers = st.slider("Min papers to analyze", 3, 20, 5)

    if st.button("🔍 Analyze Gaps", type="primary") and topic.strip():
        with st.spinner("Analyzing gaps..."):
            result = analyzer.analyze(
                topic=topic.strip(),
                min_papers=min_papers,
                use_llm=False,  # Skip LLM for speed
            )

        st.success(f"Found {len(result.gaps)} gaps from {result.total_papers_analyzed} papers")

        if result.gaps:
            st.subheader(f"Ranked Gaps (top {min(len(result.gaps), 20)})")
            for i, gap in enumerate(result.gaps[:20]):
                # Gap type name
                gap_type_name = gap.gap_type.value if hasattr(gap.gap_type, 'value') else str(gap.gap_type)
                sev_icon = "🔴" if gap.severity.value == "HIGH" else "🟡" if gap.severity.value == "MEDIUM" else "🟢"

                with st.expander(f"#{i+1} {sev_icon} {gap_type_name}: {gap.title[:60]}"):
                    col_left, col_right = st.columns([2, 1])

                    with col_left:
                        st.write(f"**Description:** {gap.description[:200]}")
                        st.write(f"**Severity:** {gap.severity.value}")
                        st.write(f"**Supporting Papers:** {len(gap.supporting_papers)}")
                        if gap.sub_questions:
                            st.write(f"**Sub-questions:** {len(gap.sub_questions)}")

                    with col_right:
                        # 6-tuple breakdown
                        st.write("**6-Tuple Scores:**")
                        st.write(f"  trend:    {gap.novelty_score:.2f}")  # reused for trend
                        st.write(f"  gene_pool: {gap.gene_pool_score:.3f}")
                        st.write(f"  pref:     {gap.preference_score:+.1f}")
                        st.write(f"  severity: {gap.severity.value}")
                        st.write(f"  priority: {gap.priority}")

                        # Preference boost indicator
                        if gap.preference_boost:
                            st.markdown("✅ **Matches your preferences**")

                        if gap.gene_pool_score > 0:
                            st.markdown(f"🧬 **Gene Pool signal: {gap.gene_pool_score:.3f}**")

                        # Record feedback
                        col_acc, col_rej = st.columns(2)
                        with col_acc:
                            if st.button(f"✅ Accept", key=f"acc_{i}"):
                                tracker.record_gap_accept(
                                    topic=topic,
                                    gap_type=gap_type_name,
                                    gap_title=gap.title,
                                    gap_description=gap.description,
                                )
                                st.success("Accepted! Gene Pool updated.")
                                st.rerun()
                        with col_rej:
                            if st.button(f"❌ Reject", key=f"rej_{i}"):
                                tracker.record_gap_reject(
                                    topic=topic,
                                    gap_type=gap_type_name,
                                    gap_title=gap.title,
                                    reason="Not useful",
                                )
                                st.warning("Rejected.")
                                st.rerun()

            # Gap type distribution
            if result.gaps_by_type:
                st.divider()
                st.subheader("Gap Type Distribution")
                types = [str(k.value) for k in result.gaps_by_type.keys()]
                counts = list(result.gaps_by_type.values())
                st.bar_chart({"Gap Type": types, "Count": counts})
        else:
            st.info("No gaps found. Try a different topic or add more papers.")
    else:
        st.info("Enter a research topic above to start gap detection.")

    st.divider()

    # Gene Pool signal explanation
    with st.expander("🧬 Gene Pool Signal Details", expanded=False):
        gp_stats = tracker.get_gene_pool_stats()
        st.json(gp_stats)
        st.markdown("""
**Gene Pool Score Calculation:**
```
s_gene(gap) = max_{c in GenePool}[ c.outcome_success_score × trigger_match(topic, gap_type, keywords) ]
```

The best matching CapsuleGene's success score is weighted by how well its trigger
pattern matches the new gap context.
        """)


# ─── InsightEvolution Page ──────────────────────────────────────────────────

elif page == "🔄 InsightEvolution":
    st.header("🔄 InsightEvolution — Feedback-Descent闭环")

    with st.expander("ℹ️ The闭环 loop", expanded=False):
        st.markdown("""
**InsightEvolution** is a four-stage feedback loop that continuously improves capsule quality:

```
┌─────────────────────────────────────────────────────┐
│  1. AUDIT   → Score all capsules on quality         │
│                    (novelty + utility + freshness)   │
│                         ↓                           │
│  2. PROPOSE  → Generate V2 candidates via mutation  │
│     (trigger_refine, gap_type_transfer,             │
│      keyword_expand, llm_suggested)                │
│                         ↓                           │
│  3. EVALUATE → Pairwise LLM comparison              │
│                         ↓                           │
│  4. APPLY    → Retire POOR, update GOOD,           │
│                adopt top candidates                 │
└─────────────────────────────────────────────────────┘
```

**CapsuleQuality thresholds:**
- EXCELLENT: score ≥ 0.8 → update toward EXCELLENT
- GOOD: score ≥ 0.6 → maintain
- FAIR: score ≥ 0.4 → monitor
- POOR: score < 0.4 → retire
        """)

    # Init
    if "evo_tracker" not in st.session_state:
        from llm.insight.tracker import EvolutionTracker
        st.session_state["evo_tracker"] = EvolutionTracker()

    tracker = st.session_state["evo_tracker"]

    if "evo_engine" not in st.session_state:
        from llm.insight.evolution import InsightEvolution
        st.session_state["evo_engine"] = InsightEvolution(tracker)

    engine = st.session_state["evo_engine"]

    # Stats overview
    c1, c2 = st.columns(2)
    gp_stats = tracker.get_gene_pool_stats()
    with c1:
        st.metric("Gene Pool Size", gp_stats["total"])
    with c2:
        st.metric("Avg Capsule Score", f"{gp_stats['avg_score']:.3f}")

    st.divider()

    # Run full cycle
    st.subheader("🚀 Run Feedback-Descent Cycle")

    with st.expander("Cycle Options", expanded=False):
        col_top, col_gtype = st.columns(2)
        with col_top:
            cycle_topic = st.text_input("Topic", value="RLHF", key="cycle_topic")
        with col_gtype:
            cycle_gtype = st.selectbox("Gap Type", [
                "method_limitation", "unexplored_application", "contradiction",
                "evaluation_gap", "scalability_issue", "theoretical_gap",
            ], key="cycle_gtype")

    if st.button("🔄 Run Full Cycle", type="primary"):
        with st.spinner("Running audit → propose → evaluate → apply..."):
            result = engine.run_full_cycle(
                tracker=tracker,
                topic=cycle_topic,
                gap_type=cycle_gtype,
                auto_accept=False,
            )

        # Display results
        st.success("Cycle complete!")

        audit: "AuditResult" = result.get("audit_result")
        if audit:
            st.json({
                "total_capsules": audit.total_capsules,
                "avg_quality": audit.avg_quality,
                "high_quality_ids": audit.candidate_ids[:5],
                "low_quality_ids": audit.retire_ids[:5],
            })

        proposals = result.get("proposals", [])
        st.write(f"**{len(proposals)} V2 candidates generated:**")
        for p in proposals[:5]:
            st.write(f"  [{p.source}] {p.mutation_description[:80]}")

    st.divider()

    # Manual audit display
    st.subheader("📊 Current Gene Pool Audit")

    if st.button("🔍 Run Audit Only"):
        capsules = []
        gp_file = tracker._gene_pool_file
        if gp_file.exists():
            import json
            with open(gp_file, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        try:
                            capsules.append(json.loads(line))
                        except Exception:
                            pass

        if capsules:
            from llm.insight.evolution import CapsuleQuality
            audit_result = engine.audit(capsules, topic="RLHF")
            st.json({
                "total_capsules": audit_result.total_capsules,
                "avg_quality": audit_result.avg_quality,
                "high_quality": len(audit_result.high_quality),
                "low_quality": len(audit_result.low_quality),
                "candidate_ids": audit_result.candidate_ids,
                "retire_ids": audit_result.retire_ids,
            })

            # Show quality breakdown
            if audit_result.high_quality:
                st.markdown("**🟢 High Quality Capsules:**")
                for q in audit_result.high_quality[:5]:
                    st.write(f"  [{q.capsule_id[:8]}] score={q.overall:.2f} (novelty={q.novelty:.2f}, utility={q.utility:.2f}, freshness={q.freshness:.2f})")

            if audit_result.low_quality:
                st.markdown("**🔴 Low Quality Capsules (candidates for retirement):**")
                for q in audit_result.low_quality[:5]:
                    st.write(f"  [{q.capsule_id[:8]}] score={q.overall:.2f}")
        else:
            st.info("No capsules to audit. Accept some gaps first!")

    st.divider()

    # Architecture diagram
    st.subheader("🏗️ System Architecture")
    st.markdown("""
```
User Action
    │
    ▼
EvolutionTracker.record_gap_accept(topic, gap_type, title)
    │
    ├─► encode_capsule() → Gene Pool (gene_pool.jsonl)
    │
    ├─► _update_profile() → UserPreferenceProfile (profile.json)
    │
    └─► _invalidate_score_cache()

GapAnalyzerV2.analyze(topic)
    │
    └─► _get_gene_pool_score(topic, gap)
            │
            └─► tracker.find_capsule(topic, gap_type, keywords)
                    │
                    └─► trigger_match() → CapsuleGene.outcome_success_score

GapAnalyzerV2._apply_preference_sorting()
    │
    └─► gap_preference_score(gap) = (trend, gene_pool, keyword, pref, severity, priority)
            │
            └─► gaps.sort(key=gap_preference_score, reverse=True)
```
""")


# ─── MCP Research Panel ──────────────────────────────────────────────────────

elif page == "🔍 MCP Research":
    st.header("🔍 MCP Research Panel — Web + Local Search")

    with st.expander("ℹ️ What can I do here?", expanded=False):
        st.markdown("""
**MCP Research Panel** gives the web UI the same research superpowers as the Claude Code MCP plugin:

- **Web Search**: Query arXiv + Semantic Scholar concurrently — no CLI needed
- **One-Click Import**: Import any web result directly to your local DB
- **Citation Graph**: Visualize forward/backward citations for any paper
- **Gap Detection**: Find research gaps using your local paper corpus

All of this mirrors what `paper_search(source=web)`, `paper_ingest`, and `citation_graph` do in the MCP server.
        """)

    tab_web, tab_graph, tab_gap = st.tabs(["🌐 Web Search", "🔗 Citation Graph", "🎯 Gap Detection"])

    # ── Web Search Tab ─────────────────────────────────────────────────────────
    with tab_web:
        col_q, col_src = st.columns([4, 1])
        with col_q:
            search_query = st.text_input(
                "🔍 Search Query",
                value="RLHF reinforcement learning",
                placeholder="Enter research topic, e.g. 'RLHF', 'attention mechanism'",
                key="mcp_search_query",
            )
        with col_src:
            search_source = st.selectbox(
                "Source",
                ["web", "local", "both"],
                index=0,
                format_func=lambda x: {"web": "🌐 Web Only", "local": "💾 Local Only", "both": "🔄 Both"}[x],
                key="mcp_search_source",
            )

        if st.button("🔍 Search", type="primary", key="mcp_do_search"):
            if not search_query.strip():
                st.warning("Enter a search query first.")
            else:
                with st.spinner("Searching..."):
                    results = []
                    sources_used = []

                    if search_source in ("local", "both"):
                        try:
                            db = _get_db()
                            local_results, total = db.search_papers(search_query.strip(), limit=20)
                            db.close()
                            for r in local_results:
                                results.append({
                                    "paper_id": r.paper_id,
                                    "title": r.title,
                                    "authors": r.authors or [],
                                    "published": r.published or "",
                                    "source": "local",
                                    "abs_url": getattr(r, 'abs_url', '') or "",
                                    "abstract": getattr(r, 'abstract', '') or "",
                                })
                            sources_used.append(f"local({total})")
                        except Exception as e:
                            st.warning(f"Local search failed: {e}")

                    if search_source in ("web", "both"):
                        try:
                            from parsers.cross_search import search_papers_multi
                            web_papers = search_papers_multi(search_query.strip(), max_per_source=10)
                            for p in web_papers:
                                results.append({
                                    "paper_id": p.uid,
                                    "title": p.title,
                                    "authors": p.authors or [],
                                    "published": p.published[:10] if p.published else "",
                                    "source": getattr(p, 'source', 'web'),
                                    "abs_url": getattr(p, 'abs_url', '') or "",
                                    "abstract": getattr(p, 'abstract', '') or "",
                                })
                            sources_used.append(f"web({len(web_papers)})")
                        except Exception as e:
                            st.warning(f"Web search failed: {e}. Check API rate limits.")

                if results:
                    st.success(f"Found {len(results)} results from {' + '.join(sources_used)}")
                    st.session_state["mcp_search_results"] = results
                else:
                    st.warning("No results found. Try a different query.")

        # Display cached results
        cached = st.session_state.get("mcp_search_results", [])
        if cached:
            st.divider()
            st.subheader(f"Results ({len(cached)} papers)")
            for r in cached:
                src_color = "🟢" if r["source"] == "local" else "🔵"
                with st.expander(f"{src_color} **{r['title'][:80]}**"):
                    authors = ", ".join(r.get("authors", [])[:5]) if r.get("authors") else "N/A"
                    st.write(f"**Authors:** {authors}")
                    st.write(f"**Year:** {r.get('published', '?')[:4]} | **Source:** {r.get('source', '?')}")
                    if r.get("abstract"):
                        st.write(f"**Abstract:** {r['abstract'][:300]}...")
                    if r.get("abs_url"):
                        st.markdown(f"[View Paper]({r['abs_url']})")

                    # One-click import button
                    pid = r.get("paper_id", "")
                    if pid:
                        col_imp, col_graph = st.columns([1, 1])
                        with col_imp:
                            if st.button(f"📥 Import to DB", key=f"imp_{pid}"):
                                try:
                                    db = _get_db()
                                    if r["source"] != "local":
                                        if pid.startswith("10."):
                                            from parsers.crossref import fetch_crossref_metadata
                                            paper_obj, _ = fetch_crossref_metadata(pid)
                                        else:
                                            from parsers.arxiv import fetch_arxiv_metadata
                                            paper_obj = fetch_arxiv_metadata(pid)
                                    else:
                                        paper_obj = None

                                    if paper_obj:
                                        db.upsert_paper(
                                            paper_id=pid,
                                            source=r["source"],
                                            title=paper_obj.title or r.get("title", ""),
                                            authors=paper_obj.authors or r.get("authors", []),
                                            abstract=paper_obj.abstract or r.get("abstract", ""),
                                            published=paper_obj.published or r.get("published", ""),
                                            abs_url=paper_obj.abs_url or r.get("abs_url", ""),
                                            pdf_url=getattr(paper_obj, "pdf_url", "") or "",
                                            primary_category=getattr(paper_obj, "primary_category", "") or "",
                                            doi=getattr(paper_obj, "doi", "") or "",
                                        )
                                    st.success(f"✅ Imported: {r.get('title', pid)[:60]}")
                                except Exception as e:
                                    st.error(f"❌ Import failed: {e}")

                        with col_graph:
                            if st.button(f"🔗 View Graph", key=f"graph_{pid}"):
                                st.session_state["mcp_graph_paper"] = pid
                                st.rerun()
        else:
            st.info("Enter a query above and click **Search** to find papers.")

    # ── Citation Graph Tab ────────────────────────────────────────────────────
    with tab_graph:
        graph_pid = st.text_input(
            "Paper ID (arXiv ID or DOI)",
            value=st.session_state.get("mcp_graph_paper", "2307.02486"),
            key="mcp_graph_pid_input",
        )
        col_depth, col_nodes = st.columns([1, 1])
        with col_depth:
            graph_depth = st.selectbox("Depth", [1, 2, 3], index=1, key="mcp_graph_depth")
        with col_nodes:
            max_nodes = st.slider("Max nodes/direction", 10, 100, 30, key="mcp_max_nodes")

        if st.button("🔗 Load Citation Graph", type="primary", key="mcp_load_graph"):
            if not graph_pid.strip():
                st.warning("Enter a paper ID.")
            else:
                with st.spinner("Fetching citation data from Semantic Scholar..."):
                    try:
                        from parsers.semantic_scholar import get_paper_by_id, get_citations, get_references
                        root = get_paper_by_id(graph_pid.strip())
                        if not root:
                            st.error(f"Paper not found: {graph_pid}")
                        else:
                            nodes = []
                            links = []
                            seen = set()

                            nodes.append({
                                "id": root.paper_id,
                                "label": root.title,
                                "is_root": True,
                                "is_citing": False,
                                "is_cited_by": False,
                                "type": "Paper",
                                "entity_id": root.paper_id,
                            })
                            seen.add(root.paper_id)

                            try:
                                citing = get_references(root.paper_id, limit=max_nodes)
                                for p in citing:
                                    if p.paper_id not in seen:
                                        nodes.append({
                                            "id": p.paper_id,
                                            "label": p.title,
                                            "is_root": False,
                                            "is_citing": True,
                                            "is_cited_by": False,
                                            "type": "Paper",
                                            "entity_id": p.paper_id,
                                        })
                                        seen.add(p.paper_id)
                                    links.append({
                                        "source": root.paper_id,
                                        "target": p.paper_id,
                                        "relation": "cites",
                                        "weight": 1,
                                    })
                            except Exception as e:
                                st.warning(f"Could not fetch citing papers: {e}")

                            try:
                                cited_by = get_citations(root.paper_id, limit=max_nodes)
                                for p in cited_by:
                                    if p.paper_id not in seen:
                                        nodes.append({
                                            "id": p.paper_id,
                                            "label": p.title,
                                            "is_root": False,
                                            "is_citing": False,
                                            "is_cited_by": True,
                                            "type": "Paper",
                                            "entity_id": p.paper_id,
                                        })
                                        seen.add(p.paper_id)
                                    links.append({
                                        "source": p.paper_id,
                                        "target": root.paper_id,
                                        "relation": "cited_by",
                                        "weight": 1,
                                    })
                            except Exception as e:
                                st.warning(f"Could not fetch cited-by papers: {e}")

                            st.session_state["mcp_citation_data"] = {
                                "nodes": nodes,
                                "links": links,
                                "root": root.title,
                            }
                            st.session_state["mcp_graph_paper"] = graph_pid.strip()
                    except Exception as e:
                        st.error(f"Failed to load citation graph: {e}")

        # Render D3 graph
        citation_data = st.session_state.get("mcp_citation_data")
        if citation_data:
            import json
            graph_json = json.dumps(citation_data)
            st.success(f"Loaded {len(citation_data['nodes'])} nodes, {len(citation_data['links'])} links")
            st.markdown(f"**Root paper:** {citation_data.get('root', graph_pid)}")

            st.components.v1.html(f"""
            <div id="mcp_cit_graph" style="width:100%;height:500px;border:1px solid #30363d;border-radius:8px;margin-top:8px;"></div>
            <script src="https://unpkg.com/d3@7/dist/d3.min.js"></script>
            <script>
            const data = {graph_json};
            const color = {{ root: '#4A90E2', cites: '#58a6ff', cited_by: '#f78166' }};
            const w = document.getElementById('mcp_cit_graph').clientWidth || 800;
            const h = 500;
            d3.select('#mcp_cit_graph').selectAll('*').remove();
            const svg = d3.select('#mcp_cit_graph').append('svg').attr('width', w).attr('height', h);
            const g = svg.append('g');
            svg.call(d3.zoom().scaleExtent([0.1, 4]).on('zoom', e => g.attr('transform', e.transform)));
            const simulation = d3.forceSimulation(data.nodes)
                .force('link', d3.forceLink(data.links).id(d => d.id).distance(80))
                .force('charge', d3.forceManyBody().strength(-200))
                .force('center', d3.forceCenter(w/2, h/2))
                .force('collision', d3.forceCollide().radius(30));
            const link = g.append('g').selectAll('line').data(data.links).join('line')
                .attr('stroke', d => d.relation === 'cites' ? color.cites : color.cited_by)
                .attr('stroke-width', 1.5).attr('stroke-opacity', 0.6);
            const node = g.append('g').selectAll('g').data(data.nodes).join('g');
            node.append('circle').attr('r', d => d.is_root ? 10 : 6)
                .attr('fill', d => d.is_root ? color.root : d.is_citing ? color.cites : color.cited_by);
            node.append('text').attr('dx', 12).attr('dy', 4).attr('font-size', '10px').attr('fill', '#e6edf3')
                .text(d => (d.label||'').substring(0,25));
            simulation.on('tick', () => {{
                link.attr('x1', d=>d.source.x).attr('y1',d=>d.source.y)
                    .attr('x2',d=>d.target.x).attr('y2',d=>d.target.y);
                node.attr('transform', d => 'translate('+d.x+','+d.y+')');
            }});
            </script>
            """, height=520)

            citing_count = sum(1 for n in citation_data["nodes"] if n.get("is_citing"))
            cited_by_count = sum(1 for n in citation_data["nodes"] if n.get("is_cited_by"))
            col_c1, col_c2 = st.columns(2)
            col_c1.metric("Cites (forward)", citing_count)
            col_c2.metric("Cited by (backward)", cited_by_count)

            root_pid = st.session_state.get("mcp_graph_paper", "")
            if root_pid:
                col_exp, col_imp = st.columns([1, 1])
                with col_exp:
                    if st.button(f"📥 Import Root Paper to DB", key="mcp_imp_root"):
                        try:
                            db = _get_db()
                            if root_pid.startswith("10."):
                                from parsers.crossref import fetch_crossref_metadata
                                paper_obj, _ = fetch_crossref_metadata(root_pid)
                            else:
                                from parsers.arxiv import fetch_arxiv_metadata
                                paper_obj = fetch_arxiv_metadata(root_pid)
                            if paper_obj:
                                db.upsert_paper(
                                    paper_id=root_pid,
                                    source="arxiv" if not root_pid.startswith("10.") else "doi",
                                    title=paper_obj.title or "", authors=paper_obj.authors or [],
                                    abstract=paper_obj.abstract or "", published=paper_obj.published or "",
                                    abs_url=paper_obj.abs_url or "", pdf_url=getattr(paper_obj, "pdf_url", "") or "",
                                    primary_category=getattr(paper_obj, "primary_category", "") or "",
                                    doi=getattr(paper_obj, "doi", "") or "",
                                )
                                st.success(f"✅ Imported: {paper_obj.title[:60]}")
                        except Exception as e:
                            st.error(f"❌ {e}")
                with col_imp:
                    graph_json_str = json.dumps(citation_data)
                    st.download_button(
                        "📤 Export Graph JSON",
                        graph_json_str,
                        file_name=f"citation_graph_{root_pid}.json",
                        mime="application/json",
                        key="mcp_export_graph",
                    )
        else:
            st.info("Enter a paper ID and click **Load Citation Graph** to visualize citations.")

    # ── Gap Detection Tab ──────────────────────────────────────────────────────
    with tab_gap:
        st.subheader("🎯 Research Gap Detection")
        gap_topic = st.text_input(
            "Research Topic",
            value="RLHF",
            placeholder="e.g. RLHF, RAG, Diffusion Models",
            key="mcp_gap_topic",
        )
        min_papers_gap = st.slider("Min papers to analyze", 3, 20, 5, key="mcp_gap_papers")

        if st.button("🔍 Detect Gaps", type="primary", key="mcp_detect_gaps"):
            if not gap_topic.strip():
                st.warning("Enter a research topic.")
            else:
                with st.spinner("Analyzing papers for research gaps..."):
                    try:
                        if "gap_analyzer_v2" not in st.session_state:
                            from llm.insight.tracker import EvolutionTracker
                            from llm.gap_analyzer import GapAnalyzerV2
                            tracker = EvolutionTracker()
                            st.session_state["gap_analyzer_v2"] = GapAnalyzerV2(evolution_tracker=tracker)

                        analyzer = st.session_state["gap_analyzer_v2"]
                        result = analyzer.analyze(
                            topic=gap_topic.strip(),
                            min_papers=min_papers_gap,
                            use_llm=False,
                        )

                        if result.gaps:
                            st.session_state["mcp_gap_results"] = result.gaps
                            st.success(f"Found {len(result.gaps)} gaps from {result.total_papers_analyzed} papers")
                        else:
                            st.session_state["mcp_gap_results"] = []
                            st.info("No gaps found. Try a different topic or more papers.")
                    except Exception as e:
                        st.error(f"Gap detection failed: {e}")

        gap_results = st.session_state.get("mcp_gap_results", [])
        if gap_results:
            st.subheader(f"Detected Gaps ({len(gap_results)})")
            for i, gap in enumerate(gap_results[:20]):
                gap_type_name = gap.gap_type.value if hasattr(gap.gap_type, 'value') else str(gap.gap_type)
                sev_icon = "🔴" if gap.severity.value == "HIGH" else "🟡" if gap.severity.value == "MEDIUM" else "🟢"
                with st.expander(f"#{i+1} {sev_icon} {gap_type_name}: {gap.title[:60]}"):
                    st.write(f"**Description:** {gap.description[:200]}")
                    st.write(f"**Severity:** {gap.severity.value} | **Supporting Papers:** {len(gap.supporting_papers)}")
                    if gap.gene_pool_score > 0:
                        st.markdown(f"🧬 **Gene Pool signal: {gap.gene_pool_score:.3f}**")
                    if gap.preference_boost:
                        st.markdown("✅ **Matches your preferences**")
        else:
            st.info("Enter a topic and click **Detect Gaps** to find research gaps.")


# ─── Autopilot Watch Page ─────────────────────────────────────────────────────

elif page == "🚀 Autopilot Watch":
    st.header("🚀 Autopilot Watch — Autonomous Research Agent")

    with st.expander("ℹ️ How does it work?", expanded=False):
        st.markdown("""
**Autopilot Watch** runs the closed loop:

```
arXiv subscription watch
    → new papers detected
        → DeepResearchAgent (gap analysis loop)
            → GapAnalyzerV2 (gap detection)
                → Gene Pool scoring (preference-aware ranking)
                    → Discord/Feishu webhook notification
                    → Gene Pool encoding
```

You control it via Claude Code (`research_agent_start/stop/status/trigger`)
or here in the dashboard.
        """)

    # Init orchestrator
    from research_loop.orchestrator import AutonomousOrchestrator, _load_state
    orch = AutonomousOrchestrator(webhook_enabled=True)

    # Status display
    status = orch.get_status()
    col1, col2, col3, col4 = st.columns(4)
    col1.metric("Running", "✅ Yes" if status["running"] else "❌ No")
    col2.metric("Interval", f"{status['interval_minutes']} min")
    col3.metric("Last Check", status["last_check"] or "never")
    col4.metric("Alerts Stored", status["alerts_count"])

    st.divider()

    # Control buttons
    col_start, col_trigger, col_stop = st.columns(3)
    with col_start:
        interval = st.selectbox("Check Interval", [15, 30, 60, 120], index=1, key="autopilot_interval")
        if st.button("🚀 Start Watch", type="primary"):
            orch.start_watch(interval_minutes=interval)
            st.success("Watch started! It will run in the background.")
            st.rerun()

    with col_trigger:
        st.write("")
        st.write("")
        if st.button("⚡ Trigger One Cycle Now"):
            with st.spinner("Running research cycle..."):
                try:
                    alerts = orch.run_cycle()
                    st.success(f"Cycle complete: {len(alerts)} alerts generated")
                    st.rerun()
                except Exception as e:
                    st.error(f"Cycle failed: {e}")

    with col_stop:
        st.write("")
        st.write("")
        if st.button("⏹ Stop Watch"):
            orch.stop_watch()
            st.warning("Watch stopped.")
            st.rerun()

    st.divider()

    # Recent alerts
    st.subheader("📡 Recent Research Alerts")
    alerts = orch.get_recent_alerts(limit=20)

    if alerts:
        for alert in alerts:
            sev_color = "🔴" if alert.severity == "HIGH" else "🟡" if alert.severity == "MEDIUM" else "🟢"
            with st.expander(f"{sev_color} [{alert.alert_id}] {alert.top_gap_type}: {alert.top_gap_title[:50]}"):
                col_a, col_b = st.columns([2, 1])
                with col_a:
                    st.write(f"**Topic:** {alert.topic}")
                    st.write(f"**Trigger:** [{alert.trigger_title[:60]}](https://arxiv.org/abs/{alert.triggered_by})")
                    st.write(f"**Gap:** {alert.top_gap_title[:100]}")
                    st.write(f"**Gene Pool Score:** {alert.gene_pool_score:.3f}")
                with col_b:
                    st.write(f"**Severity:** {alert.severity}")
                    st.write(f"**Time:** {time.strftime('%Y-%m-%d %H:%M', time.localtime(alert.created_at))}")
                    if alert.preference_boost:
                        st.markdown("✅ **Preference Match**")
    else:
        st.info("No alerts yet. Start the watch or trigger a cycle to discover research gaps.")

    st.divider()

    # Architecture summary
    with st.expander("🏗️ Architecture", expanded=False):
        st.markdown("""
**Components:**
- `research_loop/orchestrator.py`: `AutonomousOrchestrator` — main loop
- `llm/subscription_monitor.py`: `SubscriptionMonitor` — arXiv watch
- `llm/gap_analyzer.py`: `GapAnalyzerV2` — gap detection
- `llm/insight/tracker.py`: `EvolutionTracker` — Gene Pool

**MCP Tools:** `research_agent_start`, `research_agent_stop`, `research_agent_status`, `research_agent_trigger`
        """)


# ─── Hypothesis Lab Page ───────────────────────────────────────────────────────

elif page == "🎯 Hypothesis Lab":
    st.header("🎯 Hypothesis Lab — Gap → Hypothesis → Experiment")

    with st.expander("ℹ️ The full pipeline", expanded=False):
        st.markdown("""
**Hypothesis Lab** closes the loop from research gap to experiment:

```
Research Gap (from Gap Detection or Autopilot)
    → Generate hypothesis + experiment design (HypothesisGenerator)
        → Record experiment results
            → Compute verdict (validated / rejected / mixed)
                → Encode back to Gene Pool (preference learning)
```

**Verdict logic:**
- VALIDATED: all linked experiments succeeded
- REJECTED: all linked experiments failed
- MIXED: both successes and failures exist
- INCONCLUSIVE: no experiments yet
        """)

    tab_gen, tab_experiments, tab_board = st.tabs(["🧪 Generate", "📋 Experiments", "📊 Leaderboard"])

    # ── Generate Tab ─────────────────────────────────────────────────────────
    with tab_gen:
        st.subheader("Generate Hypotheses from Gap")
        col_topic, col_gtype = st.columns([3, 1])
        with col_topic:
            hyp_topic = st.text_input("Research Topic", value="RLHF", key="hyp_topic")
        with col_gtype:
            hyp_gtype = st.selectbox(
                "Gap Type",
                ["method_limitation", "unexplored_application", "contradiction",
                 "evaluation_gap", "scalability_issue"],
                key="hyp_gtype",
            )
        hyp_context = st.text_area(
            "Gap Context (optional)",
            placeholder="Describe the specific gap, e.g. '现有RLHF方法在分布式场景下扩展性差'",
            key="hyp_context",
        )
        creative_mode = st.checkbox("🌀 Creative cross-domain hypotheses", key="hyp_creative")

        if st.button("🧪 Generate Hypotheses", type="primary", key="hyp_generate_btn"):
            if not hyp_topic.strip():
                st.warning("Enter a research topic.")
            else:
                with st.spinner("Generating hypotheses..."):
                    try:
                        from llm.hypothesis_generator import HypothesisGenerator
                        gen = HypothesisGenerator()
                        result = gen.generate(
                            topic=hyp_topic.strip(),
                            gap_context=hyp_context.strip(),
                            use_llm=True,
                            creative=creative_mode,
                        )
                        st.session_state["hyp_results"] = result
                        st.session_state["hyp_generated"] = True
                        st.success(f"Generated {len(result.hypotheses)} hypotheses")
                    except Exception as e:
                        st.error(f"Generation failed: {e}")

        # Display generated hypotheses
        if st.session_state.get("hyp_generated"):
            result = st.session_state.get("hyp_results")
            if result and result.hypotheses:
                for i, h in enumerate(result.hypotheses):
                    hyp_id = h.id or f"h{i}"
                    sev_icon = "🟢" if h.feasibility_score > 0.6 else "🟡" if h.feasibility_score > 0.4 else "🔴"
                    novelty_bar = "🟢" if h.novelty_score > 0.6 else "🟡" if h.novelty_score > 0.4 else "🔴"

                    with st.expander(
                        f"{sev_icon} [{hyp_id[:8]}] {h.hypothesis_type.value.upper()}: {h.title[:50]}"
                    ):
                        col_left, col_right = st.columns([2, 1])
                        with col_left:
                            st.write(f"**Core Statement:** {h.core_statement}")
                            st.write(f"**Based on:** {h.based_on}")
                            if h.risk_assessment:
                                st.write(f"**Technical Risk:** {h.risk_assessment.technical_risk.value}")
                                st.write(f"**Hypothesis Risk:** {h.risk_assessment.hypothesis_risk.value}")
                        with col_right:
                            st.write(f"**Novelty:** {novelty_bar} {h.novelty_score:.2f}")
                            st.write(f"**Feasibility:** {sev_icon} {h.feasibility_score:.2f}")

                        st.divider()
                        st.write("**Experiment Design:**")
                        ed = h.experiment_design
                        st.write(f"  Baseline: {ed.baseline}")
                        st.write(f"  Variables: {', '.join(ed.variables[:3])}")
                        st.write(f"  Controls: {', '.join(ed.controls[:2])}")
                        st.write(f"  Metrics: {', '.join(ed.evaluation_metrics[:2])}")

                        # Record experiment buttons
                        col_acc, col_rej = st.columns(2)
                        with col_acc:
                            exp_name = st.text_input(
                                "Experiment name",
                                value=f"exp-{hyp_id[:6]}",
                                key=f"expname_{hyp_id}",
                            )
                            if st.button(f"✅ Record Validated", key=f"val_{hyp_id}"):
                                try:
                                    from llm.experiment_tracker import ExperimentTracker
                                    tracker = ExperimentTracker()
                                    exp = tracker.run(
                                        name=exp_name or f"exp-{hyp_id[:6]}",
                                        hypothesis_id=hyp_id,
                                    )
                                    tracker.complete(exp.id, results={"verdict": "validated"})
                                    st.success("✅ Experiment recorded as VALIDATED")
                                except Exception as e:
                                    st.error(f"Failed: {e}")
                        with col_rej:
                            if st.button(f"❌ Record Rejected", key=f"rej_{hyp_id}"):
                                try:
                                    from llm.experiment_tracker import ExperimentTracker
                                    tracker = ExperimentTracker()
                                    exp = tracker.run(
                                        name=exp_name or f"exp-{hyp_id[:6]}",
                                        hypothesis_id=hyp_id,
                                    )
                                    tracker.complete(exp.id, results={"verdict": "rejected"})
                                    st.warning("❌ Experiment recorded as REJECTED")
                                except Exception as e:
                                    st.error(f"Failed: {e}")

    # ── Experiments Tab ──────────────────────────────────────────────────────
    with tab_experiments:
        st.subheader("Experiment Records")
        try:
            from llm.experiment_tracker import ExperimentTracker
            tracker = ExperimentTracker()
            experiments = tracker.list_experiments()

            if experiments:
                st.write(f"**{len(experiments)}** experiment(s) total")
                for e in sorted(experiments, key=lambda x: x.created_at, reverse=True)[:30]:
                    icon = {"running": "⚡", "completed": "✅", "failed": "❌"}.get(e.status, "•")
                    with st.expander(f"{icon} [{e.id[:8]}] {e.name} (hypothesis: {e.hypothesis_id[:8]})"):
                        st.write(f"**Status:** {e.status}")
                        st.write(f"**Created:** {e.created_at}")
                        if e.results:
                            st.json(e.results)
            else:
                st.info("No experiments recorded yet. Generate hypotheses and record results above.")
        except Exception as e:
            st.error(f"Failed to load experiments: {e}")

    # ── Leaderboard Tab ──────────────────────────────────────────────────────
    with tab_board:
        st.subheader("Hypothesis Leaderboard")
        try:
            from llm.insight.evolution import EvolutionTracker
            from llm.experiment_tracker import ExperimentTracker

            ev = EvolutionTracker()
            tracker = ExperimentTracker()

            events = ev.get_recent_events(limit=10000)
            hypothesis_ids = set()
            for e in events:
                if e.hypothesis_id:
                    hypothesis_ids.add(e.hypothesis_id)

            experiments = tracker.list_experiments()
            exp_by_hid = {}
            for e in experiments:
                if e.hypothesis_id:
                    exp_by_hid.setdefault(e.hypothesis_id, []).append(e)

            verdict_order = {"VALIDATED": 0, "MIXED": 1, "REJECTED": 2, "INCONCLUSIVE": 3}
            rows = []
            for hid in sorted(hypothesis_ids):
                evts = ev.get_hypothesis_events(hid)
                verdict, detail = _compute_verdict_from_events(evts)
                linked = exp_by_hid.get(hid, [])
                rows.append((verdict_order.get(verdict, 99), verdict, detail, hid, len(linked)))

            rows.sort()
            verdict_icons = {"VALIDATED": "✅", "REJECTED": "❌", "MIXED": "⚠️", "INCONCLUSIVE": "○"}
            verdict_colors = {"VALIDATED": "green", "REJECTED": "red", "MIXED": "yellow", "INCONCLUSIVE": "gray"}

            for _, verdict, detail, hid, n_exp in rows[:30]:
                icon = verdict_icons.get(verdict, "?")
                with st.expander(f"{icon} [{hid[:8]}] {verdict} — {n_exp} experiment(s)"):
                    st.write(f"**Verdict:** {verdict}")
                    st.write(f"**Reason:** {detail}")
                    st.write(f"**Linked experiments:** {n_exp}")
        except Exception as e:
            st.error(f"Failed to load leaderboard: {e}")


    def _compute_verdict_from_events(events):
        """Compute verdict from hypothesis events."""
        if not events:
            return "INCONCLUSIVE", "no experiments recorded"
        action_vals = {e.action.value if hasattr(e.action, 'value') else str(e.action) for e in events}
        has_completed = "validated" in action_vals or "completed" in action_vals
        has_failed = "rejected" in action_vals or "failed" in action_vals
        if has_completed and has_failed:
            return "MIXED", "both validated and rejected experiments exist"
        if has_completed:
            return "VALIDATED", "all experiments succeeded"
        if has_failed:
            return "REJECTED", "all experiments failed"
        return "INCONCLUSIVE", "no completed experiments yet"


# ─── Lit Review Page ───────────────────────────────────────────────────────────

elif page == "📚 Lit Review":
    st.header("📚 Literature Review — LLM-Generated Research Summaries")

    st.markdown("""
    **Lit Review** generates narrative, structured literature reviews using an LLM.
    Enter a research topic — it pulls papers from your local database and synthesizes
    thematic sections, consensus points, and key papers.
    """)

    col_gen, col_list = st.tabs(["⚡ Generate Review", "📋 Saved Reviews"])

    with col_gen:
        topic = st.text_input("Research Topic", placeholder="e.g., retrieval-augmented generation in LLMs")
        limit = st.slider("Papers to include", 5, 50, 20)
        use_llm = st.checkbox("Use LLM for generation (uncheck for template only)", value=True)

        if st.button("🔍 Generate Literature Review", type="primary"):
            if not topic:
                st.warning("Enter a research topic")
            else:
                with st.spinner("Generating literature review..."):
                    try:
                        from llm.litreview_generator import LitReviewGenerator
                        generator = LitReviewGenerator()
                        result = generator.generate(
                            topic=topic,
                            limit=limit,
                            use_llm=use_llm,
                            output_dir=PROJECT_ROOT / "data" / "litreviews",
                        )
                        if result.success:
                            st.success(f"Review generated ({result.review.total_papers} papers, {len(result.review.sections)} sections)")
                            st.markdown(result.markdown)
                        else:
                            st.error(f"Failed: {result.error}")
                    except Exception as e:
                        st.error(f"Error: {e}")

    with col_list:
        litreview_dir = PROJECT_ROOT / "data" / "litreviews"
        reviews = []
        if litreview_dir.exists():
            for f in sorted(litreview_dir.glob("litreview_*.md"), reverse=True):
                content = f.read_text(encoding="utf-8")
                lines = content.split("\n")
                date = ""
                for line in lines[1:5]:
                    if "Generated:" in line:
                        date = line.split("Generated:")[-1].strip()
                        break
                reviews.append({"file": f.name, "date": date, "size": f.stat().st_size})

        if reviews:
            for r in reviews:
                with st.expander(f"{r['file']} — {r['date']} ({r['size']} bytes)"):
                    try:
                        content = (litreview_dir / r["file"]).read_text(encoding="utf-8")
                        st.markdown(content)
                    except Exception as e:
                        st.error(f"Failed to load: {e}")
        else:
            st.info("No saved reviews yet. Generate one above!")


# ─── Research Memory Page ──────────────────────────────────────────────────────

elif page == "🧠 Research Memory":
    st.header("🧠 Research Memory — Stance Log & Anomaly Detector")

    st.markdown("""
    **Research Memory** tracks your research stance history — what you concluded, rejected, or deferred.
    When new papers arrive, it automatically detects contradictions to your prior decisions.
    """)

    tab_stances, tab_anomalies, tab_add = st.tabs(["📋 Stances", "⚠️ Anomalies", "➕ Add Stance"])

    with tab_add:
        with st.form("add_stance_form"):
            topic = st.text_input("Topic / Research Question", placeholder="e.g., Is RAG better than fine-tuning for factuality?")
            claim = st.text_area("Your Claim / Conclusion", placeholder="e.g., RAG outperforms fine-tuning on factuality tasks by 15%")
            stance_type = st.selectbox("Stance", ["supported", "rejected", "deferred", "qualified"])
            confidence = st.slider("Confidence", 0.0, 1.0, 0.6, 0.05)
            reasoning = st.text_area("Reasoning / Evidence", placeholder="What led you to this conclusion?")
            evidence_refs = st.text_input("Evidence arXiv IDs (comma-separated)", placeholder="2301.02486, 2302.10982")
            submitted = st.form_submit_button("Record Stance")

            if submitted:
                if not topic or not claim:
                    st.warning("Topic and claim are required")
                else:
                    try:
                        from llm.research_memory import ResearchMemory, StanceType
                        memory = ResearchMemory()
                        refs = [r.strip() for r in evidence_refs.split(",") if r.strip()]
                        memory.add_stance(
                            topic=topic,
                            claim=claim,
                            stance=StanceType(stance_type),
                            reasoning=reasoning,
                            evidence_refs=refs,
                            confidence=confidence,
                        )
                        st.success(f"Stance recorded! ({stance_type})")
                    except Exception as e:
                        st.error(f"Failed: {e}")

    with tab_stances:
        try:
            from llm.research_memory import ResearchMemory, StanceType
            memory = ResearchMemory()
            summary = memory.get_summary()
            st.caption(f"**{summary['total_stances']}** stances recorded")
            col1, col2, col3 = st.columns(3)
            for i, (k, v) in enumerate(summary.get("stance_breakdown", {}).items()):
                col = [col1, col2, col3][min(i, 2)]
                col.metric(k.capitalize(), v)
        except Exception as e:
            st.error(f"Error loading memory: {e}")

        st.subheader("All Stances")
        try:
            from llm.research_memory import ResearchMemory
            memory = ResearchMemory()
            stances = memory.get_stances()
            if stances:
                for s in stances:
                    with st.expander(f"[{s.stance.value.upper()}] {s.claim[:80]}"):
                        st.markdown(f"**Topic:** {s.topic}")
                        st.markdown(f"**Claim:** {s.claim}")
                        st.markdown(f"**Confidence:** {s.confidence:.0%}")
                        if s.reasoning:
                            st.markdown(f"**Reasoning:** {s.reasoning}")
                        if s.evidence_refs:
                            st.markdown(f"**Evidence:** {', '.join(s.evidence_refs)}")
                        if s.notes:
                            st.markdown(f"**Notes:** {s.notes}")
                        st.caption(f"Created: {datetime.fromtimestamp(s.created_at).isoformat()} | Updated: {datetime.fromtimestamp(s.updated_at).isoformat()}")
            else:
                st.info("No stances recorded yet. Add your first stance above!")
        except Exception as e:
            st.error(f"Error: {e}")

    with tab_anomalies:
        st.subheader("⚠️ Anomaly Alerts")
        st.markdown("Papers that **contradict** or challenge your prior stances.")
        try:
            from llm.research_memory import ResearchMemory
            memory = ResearchMemory()
            anomalies = memory.get_recent_anomalies(limit=20)
            summary = memory.get_summary()
            st.caption(f"**{summary['total_anomalies']}** total anomalies | **{summary['recent_anomalies']}** in last 24h")

            if anomalies:
                for a in anomalies:
                    color = {"high": "🔴", "medium": "🟡", "low": "🟢"}.get(a.severity.value, "⚪")
                    with st.expander(f"{color} [{a.severity.value.upper()}] {a.paper_title[:70]}"):
                        st.markdown(f"**Anomaly type:** {a.anomaly_type}")
                        st.markdown(f"**Paper:** [{a.paper_arxiv_id}](https://arxiv.org/abs/{a.paper_arxiv_id}) — {a.paper_title[:100]}")
                        st.markdown(f"**Challenges stance:** {a.stance_claim[:100]}")
                        st.markdown(f"**Description:** {a.description}")
                        st.caption(f"Detected: {datetime.fromtimestamp(a.created_at).isoformat()}")
            else:
                st.info("No anomalies detected yet. Stances will be checked against new papers!")
        except Exception as e:
            st.error(f"Error: {e}")


# ─── Auto Reviewer Page ────────────────────────────────────────────────────────

elif page == "🎯 Auto Reviewer":
    st.header("🎯 Auto Reviewer — Adversarial Paper Stress-Test")

    st.markdown("""
    **Auto Reviewer** simulates a panel of hostile peer reviewers who try to tear apart your paper.
    Run it on any paper in your database before submission to surface hidden weaknesses.
    """)

    tab_run, tab_history = st.tabs(["⚡ Run Review", "📋 Saved Reviews"])

    with tab_run:
        col1, col2 = st.columns([1, 1])
        with col2:
            arxiv_id = st.text_input("arXiv ID", placeholder="e.g. 2307.02486")
        with col1:
            persona = st.selectbox("Reviewer Persona", [
                "all", "methodology", "contributions", "clarity", "ethics"
            ])

        if st.button("🔍 Run Adversarial Review", type="primary"):
            if not arxiv_id:
                st.warning("Enter an arXiv ID")
            else:
                with st.spinner("Running adversarial review (4 reviewers simulating attacks)..."):
                    try:
                        from llm.review_simulator import ReviewSimulator, ReviewPersona, _REVIEW_PERSONAS
                        from db.database import Database

                        db = Database()
                        db.init()
                        rows, _ = db.search_papers(arxiv_id, limit=1)
                        if not rows:
                            st.error(f"Paper {arxiv_id} not found in database")
                        else:
                            row = rows[0]
                            paper_text = getattr(row, "abstract", "") or ""
                            title = getattr(row, "title", "")
                            full_text = f"{title}\n\n{paper_text}"

                            simulator = ReviewSimulator()
                            if persona != "all":
                                persona_map = {p.name.lower().split()[0]: p for p in _REVIEW_PERSONAS}
                                selected = persona_map.get(persona.lower())
                                if selected:
                                    review = simulator.review(full_text, title, persona=selected)
                                else:
                                    review = simulator.review(full_text, title)
                            else:
                                review = simulator.review(full_text, title)

                            from llm.review_simulator import save_review
                            save_review(review)

                            score_color = "🟢" if review.overall_score >= 7 else "🟡" if review.overall_score >= 5 else "🔴"
                            st.subheader(f"Overall Score: {score_color} **{review.overall_score}/10** — *{review.recommendation.upper()}*")
                            st.markdown(review.summary)

                            col_s, col_w = st.columns(2)
                            with col_s:
                                st.markdown("**✅ Strengths**")
                                for s in review.strengths:
                                    st.markdown(f"- {s}")
                            with col_w:
                                st.markdown("**❌ Weaknesses**")
                                for w in review.weaknesses:
                                    st.markdown(f"- {w}")

                            st.markdown("---")
                            st.markdown(f"**📝 Annotations ({len(review.annotations)})**")
                            sev_emoji = {"critical": "🔴", "major": "🟡", "minor": "🟢", "praise": "✨"}
                            dim_label = {
                                "methodology": "Methodology",
                                "novelty_contribution": "Novelty/Contribution",
                                "clarity_presentation": "Clarity",
                                "baselines_comparison": "Baselines",
                                "reproducibility": "Reproducibility",
                                "overclaiming": "Overclaiming",
                                "related_work": "Related Work",
                            }
                            for a in review.annotations:
                                emoji = sev_emoji.get(a.severity.value, "⚪")
                                dim = dim_label.get(a.dimension.value, a.dimension.value)
                                with st.expander(f"{emoji} [{a.severity.value.upper()}] {dim}: {a.headline}"):
                                    st.markdown(f"**Location:** {a.location}")
                                    st.markdown(f"**Comment:** {a.comment}")
                                    if a.suggestion:
                                        st.markdown(f"**💡 Suggestion:** {a.suggestion}")

                            st.success(f"Review saved (ID: {review.review_id})")

                    except Exception as e:
                        st.error(f"Failed: {e}")

    with tab_history:
        try:
            from llm.review_simulator import list_reviews
            reviews = list_reviews(limit=20)
            if reviews:
                st.write(f"**{len(reviews)}** saved reviews")
                for r in reviews:
                    emoji = "🟢" if r.get("overall_score", 5) >= 7 else "🟡" if r.get("overall_score", 5) >= 5 else "🔴"
                    with st.expander(f"{emoji} {r.get('persona','')} — Score {r.get('overall_score','?')}/10 ({r.get('recommendation','')})"):
                        st.caption(f"Review ID: {r.get('review_id','')} | Created: {r.get('created_at','')[:19]}")
                        st.markdown(f"Annotations: {r.get('annotation_count', 0)}")
            else:
                st.info("No saved reviews yet. Run a review above!")
        except Exception as e:
            st.error(f"Error: {e}")


# ─── Route Planner Page ────────────────────────────────────────────────────────

elif page == "🗺️ Route Planner":
    st.header("🗺️ Route Planner — Research GPS")

    st.markdown("""
    **Route Planner** creates a concrete, dependency-ordered research plan from a hypothesis.
    Not a to-do list — a directed graph of what to do and when, with automatic re-planning on failure.
    """)

    tab_create, tab_plans = st.tabs(["➕ Create Plan", "📋 Active Plans"])

    with tab_create:
        with st.form("create_plan_form"):
            hypothesis = st.text_area("Research Hypothesis", placeholder="e.g., Retrieval-augmented generation outperforms fine-tuning on factual recall tasks")
            goal = st.text_input("Goal", placeholder="e.g., Determine whether RAG is superior to fine-tuning for factual question answering")
            submitted = st.form_submit_button("🗺️ Generate Research Route")

            if submitted:
                if not hypothesis or not goal:
                    st.warning("Both hypothesis and goal are required")
                else:
                    with st.spinner("Building dependency graph..."):
                        try:
                            from llm.route_planner import RoutePlanner
                            planner = RoutePlanner()
                            plan = planner.create_plan(hypothesis=hypothesis, goal=goal)
                            progress = plan.get_progress()
                            st.success(f"Plan created: {len(plan.steps)} steps, ~{progress['estimated_hours']}h estimated")
                            st.json({
                                "plan_id": plan.plan_id,
                                "hypothesis": plan.hypothesis,
                                "goal": plan.goal,
                                "steps": [{"id": s.step_id, "type": s.type.value, "description": s.description, "hours": s.estimated_hours, "deps": s.dependencies or []} for s in plan.steps]
                            })
                        except Exception as e:
                            st.error(f"Failed: {e}")

    with tab_plans:
        try:
            from llm.route_planner import RoutePlanner, PlanStatus, StepStatus
            planner = RoutePlanner()
            plans = planner.list_plans(limit=20)

            if not plans:
                st.info("No plans yet. Create one above!")
            else:
                for plan in plans:
                    progress = plan.get_progress()
                    status_color = {"active": "🟡", "completed": "🟢", "abandoned": "⚫", "revised": "🔵"}.get(plan.status.value, "⚪")
                    with st.expander(f"{status_color} **[{plan.status.value}]** {plan.hypothesis[:60]}"):
                        col1, col2, col3, col4 = st.columns(4)
                        col1.metric("Steps", progress["total_steps"])
                        col2.metric("Done", f"{progress['completed']}/{progress['total_steps']}")
                        col3.metric("Hours", f"{progress['completed_hours']:.0f}/{progress['estimated_hours']:.0f}h")
                        col4.metric("Progress", f"{progress['progress_pct']:.0f}%")
                        st.caption(f"Plan ID: {plan.plan_id} | Revisions: {plan.revision_count} | Created: {plan.created_at[:19]}")

                        ready = plan.get_ready_steps()
                        if ready:
                            st.markdown("**Ready to start:**")
                            for s in ready:
                                cols = st.columns([1, 4, 1, 1])
                                cols[0].write(f"`{s.step_id}`")
                                cols[1].write(s.description[:60])
                                cols[2].write(f"{s.estimated_hours}h")
                                if cols[3].button("✓ Done", key=f"done_{s.step_id}"):
                                    planner.update_step(plan.plan_id, s.step_id, StepStatus.COMPLETED)
                                    st.rerun()

                        st.markdown("**All steps:**")
                        type_emoji = {
                            "read_paper": "📄", "run_experiment": "🧪", "compare_methods": "⚖️",
                            "write_analysis": "✍️", "survey_baselines": "🔍", "check_contradiction": "⚠️",
                            "revise_hypothesis": "🔄",
                        }
                        for s in plan.steps:
                            emoji = type_emoji.get(s.type.value, "📌")
                            status_icons = {"pending": "⏳", "in_progress": "🔄", "completed": "✅", "blocked": "🚫", "failed": "❌", "skipped": "⏭️"}
                            icon = status_icons.get(s.status.value, "❓")
                            deps_str = f" (deps: {', '.join(s.dependencies)})" if s.dependencies else ""
                            st.markdown(f"{icon} {emoji} `{s.step_id}`: {s.description[:70]}{deps_str}")

                        col_rev, col_del = st.columns(2)
                        with col_rev:
                            if st.button("🔄 Revise Plan", key=f"rev_{plan.plan_id}"):
                                reason = st.text_input("Why revise?", key=f"rev_reason_{plan.plan_id}")
                                if reason:
                                    new_plan = planner.revise_plan(plan.plan_id, reason)
                                    if new_plan:
                                        st.success(f"Revised plan created: {new_plan.plan_id}")
                                        st.rerun()
                        with col_del:
                            if st.button("🗑️ Delete", key=f"del_{plan.plan_id}"):
                                planner.delete_plan(plan.plan_id)
                                st.rerun()

        except Exception as e:
            st.error(f"Error: {e}")


# ─── Research Briefing Page ───────────────────────────────────────────────────

elif page == "📋 Research Briefing":
    st.header("📋 Research Briefing — Paper Intelligence")

    st.markdown("""
    **Research Briefing** generates structured intelligence reports for papers,
    enriched with your Gene Pool gaps and Research Memory stances.
    TL;DR, key claims, weaknesses, and a verdict on whether it validates or contradicts you.
    """)

    col_input, col_view = st.columns([1, 2])

    with col_input:
        arxiv_id = st.text_input("arXiv ID", placeholder="e.g. 2307.02486")
        use_llm = st.checkbox("Use LLM (uncheck for metadata-only)", value=True)

        if st.button("📋 Generate Briefing", type="primary"):
            if not arxiv_id:
                st.warning("Enter an arXiv ID")
            else:
                with st.spinner("Generating briefing..."):
                    try:
                        from llm.briefing_generator import BriefingGenerator
                        generator = BriefingGenerator()
                        result = generator.generate(
                            arxiv_id=arxiv_id.strip(),
                            use_llm=use_llm,
                            output_dir=PROJECT_ROOT / "data" / "briefings",
                        )
                        if result.success:
                            st.session_state["_briefing_markdown"] = result.markdown
                            st.session_state["_briefing_arxiv"] = arxiv_id
                            verdict_emoji = {"validates": "✅", "contradicts": "❌", "neutral": "⚪", "irrelevant": "🚫"}
                            emoji = verdict_emoji.get(result.briefing.verdict, "⚪")
                            st.success(f"Generated! Verdict: {emoji} **{result.briefing.verdict.upper()}**")
                        else:
                            st.error(f"Failed: {result.error}")
                    except Exception as e:
                        st.error(f"Error: {e}")

    with col_view:
        markdown = st.session_state.get("_briefing_markdown", "")
        if markdown:
            st.markdown(markdown)
        else:
            st.info("Enter an arXiv ID and generate a briefing to see it here.")


