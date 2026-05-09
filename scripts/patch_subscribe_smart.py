"""Patch subscribe.py: add smart adaptive scheduling"""
with open('cli/cmd/subscribe.py', 'r', encoding='utf-8') as f:
    c = f.read()

changes = 0

# 1. Add import
if 'from scripts.smart_scheduler' not in c:
    old = 'from cli._shared import print_success, print_error, print_info'
    new = '''from cli._shared import print_success, print_error, print_info
from scripts.smart_scheduler import compute_adaptive_interval, run_cold_start_research'''
    c = c.replace(old, new, 1)
    changes += 1
    print('Import added')

# 2. Replace interval sleep with adaptive logic
old_wait = '        stop_event.wait(timeout=interval_minutes * 60)\n\n    state["running"] = False'
new_wait = '''        # ── Smart scheduling: adapt interval to GenePool saturation ──
        decision = compute_adaptive_interval(
            base_interval_minutes=interval_minutes,
            saturation=before_stats.get("saturation", 1.0),
            n_active=before_stats.get("n_active", 0),
            has_new_papers=(total > 0),
        )
        actual_interval = decision.interval_minutes

        # Cold-start: GenePool empty, trigger proactive research
        if decision.action == "cold_start" and n_active == 0:
            print_info("[Scheduler] GenePool empty — running cold-start research")
            try:
                run_cold_start_research(db)
                _print_gene_pool_saturation("after_cold_start")
            except Exception as cs_err:
                logger.error(f"[Scheduler] Cold-start failed: {cs_err}")

        print_info(f"[Scheduler] Next check in {actual_interval:.0f}min — {decision.reason}")

        stop_event.wait(timeout=actual_interval * 60)

    state["running"] = False'''

if old_wait in c:
    c = c.replace(old_wait, new_wait, 1)
    changes += 1
    print('Adaptive wait patched')
else:
    print('WARNING: wait pattern not found')
    idx = c.find('stop_event.wait(timeout=interval_minutes * 60)')
    print(repr(c[idx:idx+100]) if idx >= 0 else 'NOT FOUND')

# 3. Update interval variable after cold-start research completes
# (the interval for next iteration should reflect decision)
# This is already handled by using actual_interval in wait()

print(f'Changes: {changes}/3')

with open('cli/cmd/subscribe.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)
print(f'File size: {len(c)}')
