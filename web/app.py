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
                    paper_id = doi
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

    # Session management in sidebar
    with st.sidebar:
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

        # Initialize ResearchChat with DB context
        if "research_chat" not in st.session_state:
            from llm.research_chat import ResearchChat
            st.session_state["research_chat"] = ResearchChat(db=db)
        rc = st.session_state["research_chat"]

        # Input
        if prompt := st.chat_input("Ask about your papers..."):
            with st.chat_message("user"):
                st.write(prompt)
            db.add_chat_message(chat_session_id, "user", prompt)

            with st.chat_message("assistant"):
                try:
                    response = rc.chat(prompt)
                except Exception as e:
                    response = (
                        f"AI call failed: {e}\n\n"
                        "To enable AI responses, set one of:\n"
                        "- `OPENAI_API_KEY` (+ optional `OPENAI_BASE_URL`)\n"
                        "- `ANTHROPIC_API_KEY`"
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
