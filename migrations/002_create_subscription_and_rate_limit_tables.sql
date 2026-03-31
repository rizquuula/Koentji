-- Rate limit intervals table
CREATE TABLE IF NOT EXISTS rate_limit_intervals (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    duration_seconds BIGINT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default intervals
INSERT INTO rate_limit_intervals (name, display_name, duration_seconds) VALUES
    ('secondly', 'Secondly', 1),
    ('minutely', 'Minutely', 60),
    ('hourly', 'Hourly', 3600),
    ('3_hourly', '3 Hourly', 10800),
    ('6_hourly', '6 Hourly', 21600),
    ('12_hourly', '12 Hourly', 43200),
    ('daily', 'Daily', 86400),
    ('weekly', 'Weekly', 604800),
    ('monthly', 'Monthly', 2592000),
    ('quarterly', 'Quarterly', 7776000),
    ('yearly', 'Yearly', 31536000)
ON CONFLICT (name) DO NOTHING;

-- Subscription types table
CREATE TABLE IF NOT EXISTS subscription_types (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    rate_limit_amount INTEGER NOT NULL DEFAULT 6000,
    rate_limit_interval_id INTEGER NOT NULL REFERENCES rate_limit_intervals(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default subscription types (all daily)
INSERT INTO subscription_types (name, display_name, rate_limit_amount, rate_limit_interval_id)
SELECT v.name, v.display_name, v.rate_limit_amount, rli.id
FROM (VALUES
    ('free', 'Free', 6000),
    ('basic', 'Basic', 15000),
    ('pro', 'Pro', 50000),
    ('enterprise', 'Enterprise', 200000)
) AS v(name, display_name, rate_limit_amount)
CROSS JOIN rate_limit_intervals rli
WHERE rli.name = 'daily'
ON CONFLICT (name) DO NOTHING;

-- Add FK columns to authentication_keys
ALTER TABLE authentication_keys
    ADD COLUMN IF NOT EXISTS subscription_type_id INTEGER REFERENCES subscription_types(id),
    ADD COLUMN IF NOT EXISTS rate_limit_interval_id INTEGER REFERENCES rate_limit_intervals(id);

-- Migrate existing subscription varchar values to FK references
UPDATE authentication_keys ak
SET subscription_type_id = st.id
FROM subscription_types st
WHERE ak.subscription = st.name
  AND ak.subscription_type_id IS NULL;

-- Set rate_limit_interval_id to daily for all existing rows that don't have one
UPDATE authentication_keys
SET rate_limit_interval_id = (SELECT id FROM rate_limit_intervals WHERE name = 'daily')
WHERE rate_limit_interval_id IS NULL;
