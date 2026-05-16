# License Conflict Audit

## Active Conflict

| Dependency | License | Project License | Status |
|------------|---------|-----------------|--------|
| `PyMuPDF` (pymupdf) | AGPL-3.0 | GPL-3.0-or-later | **Conflict** |

### Risk Assessment

**PyMuPDF** is used locally (offline) for:
- `cli/cmd/chat.py` — PDF text rendering in TUI chat export
- `extable/detector.py` — Table detection in PDF pages

**AGPL-3.0 vs GPL-3.0 conflict**: AGPL is a stronger copyleft than GPL. Incorporating AGPL-licensed code into a GPL project creates a license incompatibility.

**Mitigating factors**:
1. Both usage sites are purely local/offline — no network delivery of the modified program
2. AGPL's "network use" provision (the main additional obligation over GPL) does not apply
3. No source modification of PyMuPDF itself occurs
4. Project does not distribute binaries to third parties

**This risk is labeled `tolerable_risk`** under the project's dependency review policy.

### Resolution Options

1. **Replace PyMuPDF** with `pypdf` (BSD-3-Clause) for `cli/cmd/chat.py` PDF text extraction
2. **Replace PyMuPDF** with `pdfplumber` (MIT) for `extable/detector.py` table detection
3. **Dual-license the project** under AGPL-3.0 (would conflict with other BSD/MIT dependencies)
4. **Obtain commercial license** from Artifex (PyMuPDF vendor) — not pursued for open-source project

**Recommended**: Option 1 — `pypdf` is a BSD-3-Clause library that covers the `chat.py` use case (basic PDF text extraction). The `extable/detector.py` use case (table detection via block layout) would need `pdfplumber` or a dedicated table extraction library.

## Resolved / No Conflict

All other dependencies use GPL-compatible licenses (Apache-2.0, BSD-2/3-Clause, MIT, MPL-2.0, HPND).
