// Thin ClickHouse HTTP helper for e2e roundtrip assertions.
//
// We don't pull in a CH client lib — the HTTP interface is enough for
// `SELECT … FORMAT JSON` style verification queries.
import { CLICKHOUSE_URL } from './env';

export type AuthEventRow = {
  ts: string;
  auth_key_id: string; // CH Int64 returns as string in JSON
  auth_key: string;
  device_id: string;
  usage: number;
  remaining_after: number;
  decision: 'allowed' | 'denied';
  denial_reason: string;
  latency_us: number;
};

function chRequest(sql: string): { url: string; auth: string } {
  const parsed = new URL(CLICKHOUSE_URL);
  const database = parsed.pathname.replace(/^\//, '') || 'default';
  const user = decodeURIComponent(parsed.username);
  const password = decodeURIComponent(parsed.password);
  const target = new URL(`${parsed.protocol}//${parsed.host}/`);
  target.searchParams.set('database', database);
  void sql;
  return {
    url: target.toString(),
    auth: `Basic ${Buffer.from(`${user}:${password}`).toString('base64')}`,
  };
}

/// Run a SELECT and return the parsed `data` array.
export async function chQuery<T = Record<string, unknown>>(sql: string): Promise<T[]> {
  const { url, auth } = chRequest(sql);
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'text/plain', Authorization: auth },
    body: `${sql} FORMAT JSON`,
  });
  if (!res.ok) {
    throw new Error(`ClickHouse query failed (${res.status}): ${await res.text()}`);
  }
  const body = (await res.json()) as { data: T[] };
  return body.data;
}

/// Run a DDL/DML statement that does not return rows (no FORMAT JSON).
export async function chExec(sql: string): Promise<void> {
  const { url, auth } = chRequest(sql);
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'text/plain', Authorization: auth },
    body: sql,
  });
  if (!res.ok) {
    throw new Error(`ClickHouse exec failed (${res.status}): ${await res.text()}`);
  }
}

/// Poll until at least `min` rows match the predicate or timeout.
/// The CH sink flushes on a 1 s batch boundary, so a few hundred ms
/// of slack is usually enough.
export async function waitForAuthEvents(
  where: string,
  min: number,
  timeoutMs = 5_000,
): Promise<AuthEventRow[]> {
  const deadline = Date.now() + timeoutMs;
  let last: AuthEventRow[] = [];
  while (Date.now() < deadline) {
    last = await chQuery<AuthEventRow>(
      `SELECT toString(ts) AS ts, toString(auth_key_id) AS auth_key_id, auth_key,
              device_id, usage, remaining_after, decision, denial_reason, latency_us
         FROM auth_events
        WHERE ${where}
        ORDER BY ts`,
    );
    if (last.length >= min) return last;
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`expected >= ${min} auth_events for [${where}], got ${last.length}`);
}
