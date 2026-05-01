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
