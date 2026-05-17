/**
 * Rairos Web — Bun HTTP Server
 *
 * Serves the hand-drawn HTML frontend and proxies API calls
 * to the Rust rairos-web backend.
 */

// ─── Configuration ───────────────────────────────────────────────────────────

const API_BACKEND = process.env.RAIROS_API_URL || "http://localhost:8501";
const PORT = parseInt(process.env.RAIROS_WEB_PORT || "3000");
const STATIC_DIR = `${import.meta.dir}/static`;

// ─── MIME Types ──────────────────────────────────────────────────────────────

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
  ".ico": "image/x-icon",
};

// ─── Route Map ───────────────────────────────────────────────────────────────
// Maps URL paths to static HTML files

const ROUTES: Record<string, string> = {
  "/": "dashboard.html",
  "/dashboard": "dashboard.html",
  "/papers": "papers.html",
  "/briefing": "briefing.html",
  "/briefing/history": "briefing-history.html",
  "/daemon": "daemon.html",
  "/insights": "insights.html",
  "/citation-chain": "citation-chain.html",
  "/impact": "impact.html",
  "/reports": "reports.html",
  "/research-loop": "research-loop.html",
  "/chat": "chat.html",
  "/login": "login.html",
  "/setup": "setup.html",
  "/alerts": "alerts.html",
};

// ─── Server ───────────────────────────────────────────────────────────────────

const server = Bun.serve({
  port: PORT,
  async fetch(req: Request) {
    const url = new URL(req.url);
    const path = url.pathname;

    // ── API Proxy ───────────────────────────────────────────────────────────
    if (path.startsWith("/api/")) {
      const target = `${API_BACKEND}${path.slice(4)}${url.search}`;
      try {
        const resp = await fetch(target, {
          method: req.method,
          headers: req.headers,
          body: req.method !== "GET" && req.method !== "HEAD"
            ? await req.text()
            : undefined,
        });
        return new Response(resp.body, {
          status: resp.status,
          headers: resp.headers,
        });
      } catch (e) {
        return new Response(
          JSON.stringify({ error: "API backend unavailable" }),
          { status: 502, headers: { "Content-Type": "application/json" } }
        );
      }
    }

    // ── Static files ────────────────────────────────────────────────────────
    // Direct file access: /static/style.css → ./static/style.css
    if (path.startsWith("/static/")) {
      const filePath = `${STATIC_DIR}/${path.slice(8)}`;
      const file = Bun.file(filePath);
      if (await file.exists()) {
        const ext = filePath.substring(filePath.lastIndexOf("."));
        return new Response(file, {
          headers: { "Content-Type": MIME[ext] || "application/octet-stream" },
        });
      }
      return new Response("Not found", { status: 404 });
    }

    // ── HTML pages ──────────────────────────────────────────────────────────
    const template = ROUTES[path];
    if (template) {
      const file = Bun.file(`${STATIC_DIR}/${template}`);
      if (await file.exists()) {
        return new Response(file, {
          headers: { "Content-Type": "text/html; charset=utf-8" },
        });
      }
    }

    // ── Paper detail page ───────────────────────────────────────────────────
    if (path.startsWith("/papers/") && path.length > 8) {
      const file = Bun.file(`${STATIC_DIR}/paper.html`);
      if (await file.exists()) {
        return new Response(file, {
          headers: { "Content-Type": "text/html; charset=utf-8" },
        });
      }
    }

    // ── Gene pool pages ─────────────────────────────────────────────────────
    if (path.startsWith("/gene-pool/")) {
      const page = path.split("/").pop() || "index";
      const genePages: Record<string, string> = {
        credibility: "gene-credibility.html",
        "at-risk": "gene-at-risk.html",
        bold: "gene-bold.html",
        "evolution-log": "gene-evolution.html",
        backup: "gene-backup.html",
        io: "gene-io.html",
        "cross-domain": "gene-cross-domain.html",
      };
      const tmpl = genePages[page];
      if (tmpl) {
        const file = Bun.file(`${STATIC_DIR}/${tmpl}`);
        if (await file.exists()) {
          return new Response(file, {
            headers: { "Content-Type": "text/html; charset=utf-8" },
          });
        }
      }
    }

    // ── 404 fallback ────────────────────────────────────────────────────────
    return new Response(
      `<html><body style="font-family:sans-serif;padding:40px;text-align:center;">
        <h1>🤷 Page not found</h1>
        <p><a href="/">Go to Dashboard</a></p>
      </body></html>`,
      { status: 404, headers: { "Content-Type": "text/html; charset=utf-8" } }
    );
  },
});

console.log(`🦊 Rairos Web (Bun) running on http://localhost:${PORT}`);
console.log(`  API backend: ${API_BACKEND}`);
