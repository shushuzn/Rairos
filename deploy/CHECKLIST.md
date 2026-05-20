# Deployment Checklist

## Pre-Launch Checklist

### Day 1-2: Technical Setup

- [ ] **Stripe Account**
  - [ ] Create Stripe account at https://stripe.com
  - [ ] Verify email and complete account setup
  - [ ] Get API keys from https://dashboard.stripe.com/apikeys

- [ ] **Stripe Products Setup**
  ```bash
  # Run the setup script
  ./deploy/setup_stripe.sh <your_stripe_secret_key>
  ```
  - [ ] Note the price IDs from output
  - [ ] Update `crates/rairos-api-gateway/src/stripe.rs` with real price IDs

- [ ] **Database**
  - [ ] PostgreSQL instance running (or use Supabase/Railway)
  - [ ] Run migrations: `psql $DATABASE_URL < migrations/001_initial.sql`

- [ ] **Environment Variables**
  - [ ] Copy `.env.example` to `.env`
  - [ ] Fill in all required values:
    ```bash
    DATABASE_URL=postgres://...
    REDIS_URL=redis://...
    STRIPE_WEBHOOK_SECRET=whsec_...
    STRIPE_SECRET_KEY=sk_live_...
    ```

- [ ] **Test Basic Flow**
  ```bash
  # Build
  make build

  # Run
  RUST_LOG=info ./target/release/rairos-api-gateway

  # Test health
  curl http://localhost:8081/health

  # Test metrics
  curl http://localhost:8081/metrics
  ```

### Day 3-4: Deployment

- [ ] **Cloud Resources**
  - [ ] Choose provider (AWS/GCP/DigitalOcean/Vercel)
  - [ ] Set up PostgreSQL database
  - [ ] Set up Redis instance
  - [ ] Configure firewall (allow 8081, or use reverse proxy)

- [ ] **DNS**
  - [ ] Register domain (e.g., rairos.ai)
  - [ ] Configure DNS A record to server IP
  - [ ] Set up SSL (Let's Encrypt recommended)

- [ ] **Reverse Proxy (Nginx)**
  - [ ] Install nginx
  - [ ] Configure SSL certificate
  - [ ] Proxy pass to :8081

- [ ] **Deploy**
  - [ ] Build release binary
  - [ ] Set up systemd service or Docker
  - [ ] Enable auto-start on boot

- [ ] **Webhook Endpoint**
  - [ ] Configure Stripe webhook URL: `https://api.rairos.ai/subscription/webhook`
  - [ ] Add events: `checkout.session.completed`, `customer.subscription.*`, `invoice.payment_failed`

### Day 5: Documentation

- [ ] **API Documentation**
  - [ ] Verify Swagger UI at `/docs`
  - [ ] Update contact email in OpenAPI spec
  - [ ] Add rate limit headers to responses

- [ ] **SDK**
  - [ ] Verify Python SDK: `pip install rairos`
  - [ ] Verify JS SDK: `npm install rairos`

- [ ] **README**
  - [ ] Update with real API URL
  - [ ] Add quick start guide
  - [ ] Add pricing table

### Day 6-7: Launch

- [ ] **Content**
  - [ ] Write Gap Detection introduction blog post
  - [ ] Create GitHub repository with good README
  - [ ] Add contributing guidelines

- [ ] **Monitoring**
  - [ ] Deploy Prometheus + Grafana
  - [ ] Set up Slack alerts
  - [ ] Configure dashboards

- [ ] **Launch**
  - [ ] Announce on:
    - [ ] Twitter/X
    - [ ] Hacker News
    - [ ] Reddit (r/MachineLearning, r/programming)
    - [ ] LinkedIn
  - [ ] Post on dev communities

- [ ] **Post-Launch**
  - [ ] Monitor error rates
  - [ ] Watch for abuse
  - [ ] Collect user feedback

## Success Metrics (Day 7)

| Metric | Target |
|--------|--------|
| API uptime | 99.9% |
| API latency P99 | < 500ms |
| Signups | > 10 |
| GitHub stars | > 50 |
| Active users (7d) | > 5 |

## Emergency Rollback

If issues arise:

```bash
# Quick rollback
git revert HEAD
make build
sudo systemctl restart rairos-api-gateway
```
