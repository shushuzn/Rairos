# Rairos Web UI

Web interface for Rairos API Gateway.

## Quick Start

```bash
bun install
bun run server.ts
```

Then open http://localhost:8080

## Requirements

- Bun 1.0+
- Rairos API Gateway running on port 8081

## API Proxy

Web UI proxies API requests to `/api/v1/*` → `http://localhost:8081/api/v1/*`

## Build

```bash
bun run build
```
