-- Allow multiple rows with the same key (e.g. FREE_TRIAL shared across devices)
ALTER TABLE authentication_keys DROP CONSTRAINT IF EXISTS authentication_keys_key_key;
