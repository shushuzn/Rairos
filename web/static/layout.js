/**
 * Rairos Web — Shared Layout
 * Renders the sidebar navigation and provides API helpers.
 */

// ─── API Helper ──────────────────────────────────────────────────────────────

const API = {
  async get(path) {
    const res = await fetch(`/api${path}`);
    if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`);
    return res.json();
  },
  async post(path, body) {
    const res = await fetch(`/api${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`);
    return res.json();
  },
  async del(path) {
    const res = await fetch(`/api${path}`, { method: "DELETE" });
    if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`);
    return res;
  },
};

// ─── Sidebar Navigation ──────────────────────────────────────────────────────

const NAV_SECTIONS = [
  {
    label: "Core",
    items: [
      { icon: "📊", label: "Dashboard", href: "/", page: "dashboard" },
      { icon: "🤖", label: "Daemon", href: "/daemon", page: "daemon" },
      { icon: "📡", label: "Intel", href: "/#", page: "intel" },
      { icon: "📄", label: "Reports", href: "/reports", page: "reports" },
      { icon: "📰", label: "News", href: "/#", page: "news" },
      { icon: "📚", label: "Papers", href: "/papers", page: "papers" },
      { icon: "💬", label: "Chat", href: "/chat", page: "chat" },
    ],
  },
  {
    label: "Intelligence",
    items: [
      { icon: "📋", label: "Briefing", href: "/briefing", page: "briefing" },
      { icon: "📜", label: "History", href: "/briefing/history", page: "briefing-history" },
      { icon: "🔗", label: "Citation Chain", href: "/citation-chain", page: "citation-chain" },
      { icon: "⚡", label: "Paper2Code", href: "/#", page: "paper2code" },
      { icon: "🏆", label: "Impact Ranking", href: "/impact", page: "impact" },
      { icon: "💡", label: "Research Insights", href: "/insights", page: "insights" },
      { icon: "🛡️", label: "Source Trust", href: "/#", page: "trust-scores" },
      { icon: "⚖️", label: "Gap Credibility", href: "/gene-pool/credibility", page: "gene-pool-credibility" },
      { icon: "🚨", label: "At-Risk", href: "/gene-pool/at-risk", page: "gene-pool-at-risk" },
      { icon: "🔥", label: "Heatmap", href: "/#", page: "heatmap" },
      { icon: "⚠️", label: "Paradigm", href: "/#", page: "paradigm-alert" },
      { icon: "⚡", label: "Eval Gap", href: "/#", page: "eval-gap-alert" },
      { icon: "🔴", label: "Bold Vault", href: "/gene-pool/bold", page: "gene-pool-bold" },
      { icon: "🧬", label: "Evolution Log", href: "/gene-pool/evolution-log", page: "gene-pool-evolution-log" },
      { icon: "💾", label: "Backup", href: "/gene-pool/backup", page: "gene-pool-backup" },
      { icon: "📦", label: "Import/Export", href: "/gene-pool/io", page: "gene-pool-io" },
      { icon: "📡", label: "arXiv Channels", href: "/#", page: "arxiv-channels" },
      { icon: "🔀", label: "Cross-Domain", href: "/gene-pool/cross-domain", page: "cross-domain" },
      { icon: "🌍", label: "Climate AI", href: "/#", page: "climate-monitor" },
      { icon: "🎤", label: "Voice Capsule", href: "/#", page: "voice-capsule" },
      { icon: "🏛️", label: "Policy Impact", href: "/#", page: "policy-impact" },
      { icon: "👷", label: "Labor Track", href: "/#", page: "labor-displacement" },
      { icon: "👥", label: "Researchers", href: "/#", page: "multi-researcher" },
      { icon: "🔄", label: "Research Loop", href: "/research-loop", page: "research-loop" },
      { icon: "🎮", label: "Game Mode", href: "/#", page: "game-mode" },
      { icon: "📋", label: "Review Queue", href: "/#", page: "review-queue" },
    ],
  },
];

// ─── Render Sidebar ──────────────────────────────────────────────────────────

function renderSidebar(currentPage) {
  const sidebar = document.getElementById("sidebar");
  if (!sidebar) return;

  let html = `
    <div class="sidebar-brand">
      <img src="/static/logo.svg" alt="Rairos" style="height:32px;width:32px;vertical-align:middle;margin-right:6px;">
      <span style="vertical-align:middle;">Rairos</span>
    </div>`;

  for (const section of NAV_SECTIONS) {
    html += `<div class="nav-section"><div class="nav-section-label">${section.label}</div>`;
    for (const item of section.items) {
      const active = currentPage === item.page ? ' active' : '';
      html += `<a href="${item.href}" class="nav-item${active}">
        <span class="nav-icon">${item.icon}</span> ${item.label}</a>`;
    }
    html += `</div>`;
  }

  sidebar.innerHTML = html;
}

// ─── Set Page Title ──────────────────────────────────────────────────────────

function setTitle(title) {
  document.title = `${title} — Rairos`;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function escapeHtml(s) {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

function formatDate(d) {
  if (!d) return "";
  const date = new Date(d);
  return date.toLocaleDateString("en-US", {
    year: "numeric", month: "short", day: "numeric",
  });
}

// ─── Toggle Sidebar (mobile) ─────────────────────────────────────────────────

function toggleSidebar() {
  const btn = document.getElementById("hamburger");
  const isOpen = document.getElementById("sidebar").classList.toggle("open");
  document.getElementById("sidebar-overlay")?.classList.toggle("active");
  btn?.classList.toggle("active");
  btn?.setAttribute("aria-expanded", String(isOpen));
}

// ─── Auto-init on DOMContentLoaded ──────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  const page = document.body.getAttribute("data-page") || "dashboard";
  renderSidebar(page);
});
