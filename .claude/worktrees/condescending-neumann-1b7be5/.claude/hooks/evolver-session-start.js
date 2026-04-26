#!/usr/bin/env node
// evolver-session-start.js
// Reads recent evolution memory and injects it as context for the agent session.
// Input: stdin JSON (session context). Output: stdout JSON with agent_message.

const fs = require('fs');
const path = require('path');

function findEvolverRoot() {
  const candidates = [
    process.env.EVOLVER_ROOT,
    path.resolve(__dirname, '..', '..', '..'),
  ];
  for (const c of candidates) {
    if (c && fs.existsSync(path.join(c, 'package.json'))) {
      try {
        const pkg = JSON.parse(fs.readFileSync(path.join(c, 'package.json'), 'utf8'));
        if (pkg.name === '@evomap/evolver' || pkg.name === 'evolver') return c;
      } catch { /* skip */ }
    }
  }
  const homeSkills = path.join(require('os').homedir(), 'skills', 'evolver');
  if (fs.existsSync(path.join(homeSkills, 'package.json'))) return homeSkills;
  return null;
}

function findMemoryGraph(evolverRoot) {
  if (process.env.MEMORY_GRAPH_PATH && fs.existsSync(process.env.MEMORY_GRAPH_PATH)) {
    return process.env.MEMORY_GRAPH_PATH;
  }
  const candidates = [
    evolverRoot && path.join(evolverRoot, 'memory', 'evolution', 'memory_graph.jsonl'),
    evolverRoot && path.join(evolverRoot, 'MEMORY', 'evolution', 'memory_graph.jsonl'),
  ];
  for (const c of candidates) {
    if (c && fs.existsSync(c)) return c;
  }
  return null;
}

function readLastN(filePath, n) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    const lines = content.trim().split('\n').filter(Boolean);
    return lines.slice(-n).map(line => {
      try { return JSON.parse(line); } catch { return null; }
    }).filter(Boolean);
  } catch { return []; }
}

function formatOutcome(entry) {
  const status = entry.outcome ? entry.outcome.status : 'unknown';
  const score = entry.outcome && entry.outcome.score != null ? entry.outcome.score : '?';
  const note = entry.outcome && entry.outcome.note ? entry.outcome.note : '';
  const signals = Array.isArray(entry.signals) ? entry.signals.slice(0, 3).join(', ') : '';
  const ts = entry.timestamp ? entry.timestamp.slice(0, 10) : '';
  const icon = status === 'success' ? '+' : status === 'failed' ? '-' : '?';
  return `[${icon}] ${ts} score=${score} signals=[${signals}] ${note}`.slice(0, 200);
}

function main() {
  const evolverRoot = findEvolverRoot();
  const graphPath = findMemoryGraph(evolverRoot);

  if (!graphPath) {
    process.stdout.write(JSON.stringify({}));
    return;
  }

  const entries = readLastN(graphPath, 5);
  if (entries.length === 0) {
    process.stdout.write(JSON.stringify({}));
    return;
  }

  const successCount = entries.filter(e => e.outcome && e.outcome.status === 'success').length;
  const failCount = entries.filter(e => e.outcome && e.outcome.status === 'failed').length;

  const lines = entries.map(formatOutcome);
  const summary = [
    `[Evolution Memory] Recent ${entries.length} outcomes (${successCount} success, ${failCount} failed):`,
    ...lines,
    '',
    'Use successful approaches. Avoid repeating failed patterns.',
  ].join('\n');

  process.stdout.write(JSON.stringify({
    agent_message: summary,
    additionalContext: summary,
  }));
}

main();
