ALTER TABLE authentication_keys
    ALTER COLUMN rate_limit_daily     TYPE DOUBLE PRECISION USING rate_limit_daily::double precision,
    ALTER COLUMN rate_limit_remaining TYPE DOUBLE PRECISION USING rate_limit_remaining::double precision;

ALTER TABLE subscription_types
    ALTER COLUMN rate_limit_amount    TYPE DOUBLE PRECISION USING rate_limit_amount::double precision;
