CREATE TABLE IF NOT EXISTS auth_events (
  ts            DateTime64(3, 'UTC'),
  auth_key_id   UUID,
  auth_key      String,
  device_id     String,
  usage         Float64,
  remaining_after Float64,
  decision      Enum8('allowed' = 1, 'denied' = 2),
  denial_reason LowCardinality(String),
  latency_us    UInt32
) ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (auth_key_id, ts)
TTL toDateTime(ts) + INTERVAL 90 DAY
SETTINGS index_granularity = 8192;
