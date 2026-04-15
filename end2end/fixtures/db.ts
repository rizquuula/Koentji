import { Client } from 'pg';
import { DATABASE_URL } from './env';

export function dbClient(): Client {
  return new Client({ connectionString: DATABASE_URL });
}

export async function withDb<T>(fn: (c: Client) => Promise<T>): Promise<T> {
  const c = dbClient();
  await c.connect();
  try {
    return await fn(c);
  } finally {
    await c.end();
  }
}

export type SeededKey = {
  id: number;
  key: string;
  device_id: string;
};

export type InsertKeyInput = {
  key: string;
  device_id: string;
  subscription?: string;
  subscription_type_name?: string;
  rate_limit_daily?: number;
  rate_limit_remaining?: number;
  rate_limit_interval_name?: string;
  expired_at?: string | null;
  deleted_at?: string | null;
  username?: string;
  email?: string;
};

export async function insertKey(c: Client, input: InsertKeyInput): Promise<SeededKey> {
  const subName = input.subscription_type_name ?? input.subscription ?? 'free';
  const intervalName = input.rate_limit_interval_name ?? 'daily';

  const { rows } = await c.query(
    `
    INSERT INTO authentication_keys
      (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
       username, email, expired_at, deleted_at,
       subscription_type_id, rate_limit_interval_id)
    VALUES
      ($1, $2, $3, $4, $5, $6, $7, $8, $9,
       (SELECT id FROM subscription_types WHERE name = $10),
       (SELECT id FROM rate_limit_intervals WHERE name = $11))
    RETURNING id, key, device_id
    `,
    [
      input.key,
      input.device_id,
      input.subscription ?? subName,
      input.rate_limit_daily ?? 6000,
      input.rate_limit_remaining ?? input.rate_limit_daily ?? 6000,
      input.username ?? null,
      input.email ?? null,
      input.expired_at ?? null,
      input.deleted_at ?? null,
      subName,
      intervalName,
    ],
  );
  return rows[0];
}

export async function deleteKeyByKey(c: Client, key: string): Promise<void> {
  await c.query('DELETE FROM authentication_keys WHERE key = $1', [key]);
}

export async function deleteKeyByDevice(c: Client, device_id: string): Promise<void> {
  await c.query('DELETE FROM authentication_keys WHERE device_id = $1', [device_id]);
}

export async function truncateKeys(c: Client): Promise<void> {
  await c.query('TRUNCATE authentication_keys RESTART IDENTITY CASCADE');
}

export async function setRateLimitRemaining(
  c: Client,
  id: number,
  remaining: number,
): Promise<void> {
  await c.query(
    'UPDATE authentication_keys SET rate_limit_remaining = $1, rate_limit_updated_at = NOW() WHERE id = $2',
    [remaining, id],
  );
}

export async function upsertInterval(
  c: Client,
  name: string,
  display_name: string,
  duration_seconds: number,
): Promise<number> {
  const { rows } = await c.query(
    `
    INSERT INTO rate_limit_intervals (name, display_name, duration_seconds)
    VALUES ($1, $2, $3)
    ON CONFLICT (name) DO UPDATE SET display_name = EXCLUDED.display_name, duration_seconds = EXCLUDED.duration_seconds
    RETURNING id
    `,
    [name, display_name, duration_seconds],
  );
  return rows[0].id;
}

export async function countKeysByDevice(c: Client, device_id: string): Promise<number> {
  const { rows } = await c.query<{ count: string }>(
    'SELECT COUNT(*)::text AS count FROM authentication_keys WHERE device_id = $1',
    [device_id],
  );
  return Number(rows[0].count);
}

export async function countKeysByKey(c: Client, key: string): Promise<number> {
  const { rows } = await c.query<{ count: string }>(
    'SELECT COUNT(*)::text AS count FROM authentication_keys WHERE key = $1',
    [key],
  );
  return Number(rows[0].count);
}
