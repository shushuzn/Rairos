"""Streamlit Web UI for AI Research OS.

Run: streamlit run web/app.py
"""

import json
import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

import streamlit as st

st.set_page_config(
    page_title="AI Research OS",
    layout="wide",
    page_icon="🧠",
)

st.title("🧠 AI Research OS — Web UI")


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
    "🔍 Search Papers",
    "📈 Momentum Scores",
    "📊 KG Stats",
    "🔗 KG Graph",
    "📋 Experiment Tables",
    "📉 Trends",
])


# ─── Dashboard ─────────────────────────────────────────────────────

if page == "📊 Dashboard":
    init_kg()
    kg = st.session_state["kg"]
    stats = kg.stats()

    col1, col2, col3, col4 = st.columns(4)
    col1.metric("Total Nodes", stats["total_nodes"])
    col2.metric("Total Edges", stats["total_edges"])
    col3.metric("Papers", stats["nodes_by_type"].get("Paper", 0))
    col4.metric("Tags", stats["nodes_by_type"].get("Tag", 0))

    st.subheader("Nodes by Type")
    st.bar_chart(stats["nodes_by_type"])

    st.subheader("Edges by Relation")
    st.bar_chart(stats["edges_by_type"])

    st.subheader("Recent Activity")
    all_nodes = kg.get_all_nodes()[:20]
    for n in all_nodes:
        st.write(f"`{n['type']}` **{n['label'][:60]}**")


# ─── Search Papers ─────────────────────────────────────────────────

elif page == "🔍 Search Papers":
    init_kg()
    kg = st.session_state["kg"]

    query = st.text_input("Search query", "")
    tag_filter = st.text_input("Filter by tag", "")
    ntype = st.selectbox("Node type", ["", "Paper", "P-Note", "C-Note", "M-Note", "Tag"])

    results = []
    if tag_filter:
        results = kg.find_papers_by_tag(tag_filter)
    elif query:
        all_n = kg.get_all_nodes(node_type=ntype if ntype else None)
        q = query.lower()
        results = [n for n in all_n if q in n["label"].lower()]
    else:
        results = kg.get_all_nodes(node_type=ntype if ntype else None)

    st.write(f"**{len(results)} result(s)**")
    for n in results[:50]:
        with st.expander(f"`{n['type']}` {n['label'][:60]}"):
            st.json(n)


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
            json.dumps(export, ensure_ascii=False, indent=2),
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
        results = db.search_tables(metric=metric or None, dataset=dataset or None,
                                  model=model or None, min_value=min_val if min_val > 0 else None)
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
