# Accessibility Re-Audit - Web Templates
Date: 2026-05-10 | Scope: 17 HTML templates | WCAG 2.1 AA

## Summary
| Severity | Count |
|----------|-------|
| Critical | 12 |
| Serious  | 24 |
| Minor    | 18 |
| Total    | 54 |

---

## CRITICAL ISSUES (12)

C1. papers.html - Paper Cards Not Real Checkboxes
WCAG 4.1.2 | role=checkbox on div, Spacebar does not toggle, aria-checked missing initially
Fix: Use real input[type=checkbox] in label wrappers OR implement full ARIA checkbox pattern.

C2. papers.html - Selection Bar aria-live Conflict
WCAG 4.1.3 | gap-sel-bar has display:none inline AND JS sets it. aria-live with display:none may not register.
Fix: Move initial hide to CSS class so element stays in DOM for ARIA registration.

C3. paper_detail.html - Rigor Button Loses Accessible Name
WCAG 4.1.2 | loadRigor() replaces button with span. Screen readers lose accessible name.
Fix: Keep button, change aria-label instead of replacing element.

C4. base.html - SVG Sketch Filter aria-hidden Insufficient
WCAG 1.3.1 | aria-hidden=true on SVG element may not fully remove it from accessibility tree.
Fix: Add role=presentation alongside aria-hidden=true.

C5. login.html and setup.html - Missing lang=en
WCAG 3.1.1 | Both standalone pages use html without lang attribute.
Fix: Add lang=en to html element.

C6. papers.html - Empty State Icon Not aria-hidden
WCAG 1.1.1 | empty-state-icon div contains emoji not hidden from AT.

C7. briefing_history.html - Empty State Icon Not aria-hidden
WCAG 1.1.1 | Same issue.

C8. papers.html - contradiction-badge Title Not Accessible
WCAG 1.1.1 | title attribute unreliable for screen readers.
Fix: Add aria-label.

C9. briefing.html - Error Alert Not Announced
WCAG 4.1.3 | Error div needs role=alert or aria-live=assertive.

C10. citation_chain.html - Error Alert Not Announced
WCAG 4.1.3 | Same issue.

C11. daemon.html - Alerts Container Missing aria-live
WCAG 4.1.3 | Recent Alerts section needs aria-live=polite.

C12. insights.html - Modal Loading State Not Announced
WCAG 4.1.3 | Gap modal body needs aria-live during fetch operations.

