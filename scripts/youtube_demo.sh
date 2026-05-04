#!/bin/bash
# YouTube Demo: Gene/Capsule Self-Evolution Loop
set -e
cd /d/OpenClaw/workspace/80-PROJECTS/ai_research_os

echo "=== AI Research OS: Gene/Capsule Self-Evolution Demo ==="
echo ""

echo ">>> Step 1: Research Archetype Profile"
python -c "
from llm.insight_evolution import get_evolution_tracker
tracker = get_evolution_tracker()
arch = tracker.get_archetype()
print('Label:', arch['archetype_label'], '(' + arch['dominant'] + ')')
print('Events tracked:', arch['event_count'])
dims = arch['dimensions']
for k, v in dims.items():
    bar = '█' * int(v[1] * 10)
    print('  ' + v[2].ljust(20) + bar + ' (' + str(v[1]) + ')')
"
echo ""

echo ">>> Step 2: Gene Pool status"
python -c "
from llm.insight_evolution import get_evolution_tracker
tracker = get_evolution_tracker()
stats = tracker.get_gene_pool_stats()
print('Capsules stored:', stats['total'])
print('Avg outcome score:', round(stats['avg_score'], 2))
print('By gap type:', stats['by_gap_type'])
"
echo ""

echo ">>> Step 3: Accept a gap — triggers encode_capsule"
python -c "
from llm.insight_evolution import get_evolution_tracker
tracker = get_evolution_tracker()
before = tracker.get_gene_pool_stats()['total']
event = tracker.record_gap_accept(
    topic='RLHF',
    gap_type='improvement',
    gap_title='Reward model overoptimization in RLHF',
    gap_description='KL divergence between policy and reference grows unbounded'
)
after = tracker.get_gene_pool_stats()['total']
print('Gene Capsule encoded! Pool:', before, '->', after, 'capsules')
print('Trigger:', event.gap_title)
"
echo ""

echo ">>> Step 4: Find similar capsules"
python -c "
from llm.insight_evolution import get_evolution_tracker
tracker = get_evolution_tracker()
capsules = tracker.find_capsule('RLHF', 'improvement', min_score=0.05)
print('Found', len(capsules), 'matching capsules')
for c in capsules[:3]:
    print('  [' + c.action_gap_type + '] score=' + str(c.outcome_success_score))
    print('   keywords:', c.trigger_keywords)
"
echo ""

echo ">>> Step 5: Archetype radar chart"
python -c "
from llm.insight_evolution import get_evolution_tracker
tracker = get_evolution_tracker()
print(tracker.render_archetype_radar())
"
echo ""

echo ">>> Step 6: Gene Pool by type"
python -c "
from llm.insight_evolution import get_evolution_tracker
tracker = get_evolution_tracker()
stats = tracker.get_gene_pool_stats()
print('Total capsules:', stats['total'])
print('Avg outcome score:', round(stats['avg_score'], 2))
print('Gene pool by type:')
for gt, count in stats['by_gap_type'].items():
    print(' ', gt + ':', count)
"
echo ""

echo "=== Demo Complete ==="
echo "https://github.com/shushuzn/Rairos"
echo "Docs: memory/EVOLUTION_ROADMAP.md"
