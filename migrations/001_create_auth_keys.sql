CREATE TABLE IF NOT EXISTS authentication_keys (
    id SERIAL PRIMARY KEY,
    key VARCHAR(255) UNIQUE NOT NULL,
    device_id VARCHAR(255) NOT NULL,
    subscription VARCHAR(100),
    rate_limit_daily INTEGER NOT NULL DEFAULT 6000,
    rate_limit_remaining INTEGER NOT NULL DEFAULT 6000,
    rate_limit_updated_at TIMESTAMPTZ,
    username VARCHAR(255),
    email VARCHAR(255),
    created_by VARCHAR(255),
    updated_by VARCHAR(255),
    deleted_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expired_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_auth_keys_device_id ON authentication_keys(device_id);
CREATE INDEX IF NOT EXISTS idx_auth_keys_key ON authentication_keys(key);
