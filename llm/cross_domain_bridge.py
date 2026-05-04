"""Cross-domain bridge finder using real Gene Pool data."""
def get_bridges():
    from llm.insight.tracker import EvolutionTracker
    tracker = EvolutionTracker()
    capsules = tracker._load_capsules()
    
    bridges = []
    # Group capsules by gap type
    by_type = {}
    for c in capsules:
        t = c.action_gap_type
        if t not in by_type:
            by_type[t] = []
        by_type[t].append(c)
    
    # Find pairs from different gap types that share keywords
    types = list(by_type.keys())
    for i in range(len(types)):
        for j in range(i+1, len(types)):
            type_a = types[i]
            type_b = types[j]
            for ca in by_type[type_a][:5]:
                for cb in by_type[type_b][:5]:
                    shared = set(k.lower() for k in ca.trigger_keywords) & set(k.lower() for k in cb.trigger_keywords)
                    if len(shared) >= 2:
                        bridges.append({
                            "type_a": type_a,
                            "type_b": type_b,
                            "capsule_a": ca.action_gap_title[:60],
                            "capsule_b": cb.action_gap_title[:60],
                            "shared_keywords": list(shared)[:5],
                            "strength": round(len(shared) / max(len(set(ca.trigger_keywords + cb.trigger_keywords)), 1), 2),
                        })
    
    return bridges

def render_html(bridges):
    if not bridges:
        total = 0
        try:
            from llm.insight.tracker import EvolutionTracker
            tracker = EvolutionTracker()
            caps = tracker._load_capsules()
            total = len(caps)
            types = set(c.action_gap_type for c in caps)
        except:
            types = set()
        
        return f"""
        <div class='cross-domain'>
        <h3>No cross-domain bridges found</h3>
        <p>Gene Pool has {total} capsules across {len(types)} gap types.</p>
        <p>Bridges appear when capsules from different gap types share 2+ keywords.</p>
        </div>"""
    
    html = ['<div class="cross-domain"><h3>Cross-Domain Bridges</h3>']
    for b in bridges[:20]:
        html.append(f'<div style="border:1px solid #eee;padding:10px;margin:8px 0;border-radius:6px;">')
        html.append(f'<div style="font-size:11px;color:#888;">{b["type_a"]} ↔ {b["type_b"]} (strength={b["strength"]})</div>')
        html.append(f'<div style="font-size:13px;margin:4px 0;">{b["capsule_a"][:40]}</div>')
        html.append(f'<div style="font-size:13px;">{b["capsule_b"][:40]}</div>')
        html.append(f'<div style="font-size:11px;color:#888;">shared: {", ".join(b["shared_keywords"])}</div>')
        html.append('</div>')
    html.append('</div>')
    return "\n".join(html)
