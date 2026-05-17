-- Rairos API Gateway Database Schema
-- Migration: Add alert configurations table

-- Alert configurations table
CREATE TABLE IF NOT EXISTS alert_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    threshold_percent INT NOT NULL DEFAULT 80,
    email_alert BOOLEAN NOT NULL DEFAULT true,
    webhook_url TEXT,
    last_alerted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT valid_threshold CHECK (threshold_percent > 0 AND threshold_percent <= 100)
);

-- Index for fast user lookup
CREATE INDEX IF NOT EXISTS idx_alert_configs_user_id ON alert_configs(user_id);

-- Alert history table (for tracking sent alerts)
CREATE TABLE IF NOT EXISTS alert_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    alert_type VARCHAR(20) NOT NULL, -- 'webhook', 'email'
    threshold_percent INT NOT NULL,
    usage_percent INT NOT NULL,
    requests_used BIGINT NOT NULL,
    requests_limit BIGINT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'sent', -- 'sent', 'failed'
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for fast user lookup and date filtering
CREATE INDEX IF NOT EXISTS idx_alert_history_user_id ON alert_history(user_id);
CREATE INDEX IF NOT EXISTS idx_alert_history_created_at ON alert_history(created_at);
