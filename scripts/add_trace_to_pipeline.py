"""Patch: add lineage upsert to paper2code_integration after leaderboard block"""
with open('research_loop/paper2code_integration/__init__.py', 'r', encoding='utf-8') as f:
    content = f.read()

old = '''                        except Exception:
                            pass  # non-critical: leaderboard is best-effort

                        # Update V3 capsule fitness'''

new = '''                        except Exception:
                            pass  # non-critical: leaderboard is best-effort

                        # ── Paper-Code Lineage Tracking ─────────────────────────────────
                        # Build bidirectional trace and persist to DB for provenance queries
                        try:
                            from research_loop.code_trace import code_to_paper_trace
                            if code:
                                trace_data = code_to_paper_trace(code, content)
                                db = self._get_db()
                                if db is not None:
                                    trace_id = db.upsert_paper_code_trace(
                                        paper_id=arxiv_id,
                                        code_path=str(code_path),
                                        module_name=module_name,
                                        framework=framework,
                                        total_code_lines=trace_data["total_code_lines"],
                                        tagged_lines=trace_data["total_tagged_lines"],
                                        untagged_ranges=trace_data["untagged_ranges"],
                                        unreferenced_sources=trace_data["unreferenced_sources"],
                                        paper_section_refs=trace_data["paper_section_refs"],
                                        benchmark_pass_rate=(
                                            benchmark_result.passed /
                                            (benchmark_result.passed + benchmark_result.failed)
                                            if (benchmark_result.passed + benchmark_result.failed) > 0
                                            else 0.0
                                        ),
                                    )
                                    print(
                                        f"[paper2code] Lineage: {trace_data['total_tagged_lines']}/"
                                        f"{trace_data['total_code_lines']} lines traced "
                                        f"({len(trace_data['unreferenced_sources'])} unreferenced sources)"
                                    )
                        except Exception as e:
                            print(f"[paper2code] Lineage tracking skipped: {e}")

                        # Update V3 capsule fitness'''

if old in content:
    content = content.replace(old, new, 1)
    print('Patched pipeline OK')
else:
    print('WARNING: pattern not found')
    idx = content.find('non-critical: leaderboard')
    print(repr(content[idx:idx+200]))

with open('research_loop/paper2code_integration/__init__.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)
print(f'File size: {len(content)}')
