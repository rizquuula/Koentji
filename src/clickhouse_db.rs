include!(concat!(env!("OUT_DIR"), "/clickhouse_migrations.rs"));

pub async fn create_client() -> clickhouse::Client {
    let url = std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| {
        log::warn!("CLICKHOUSE_URL not set, defaulting to http://localhost:8123");
        "http://localhost:8123".to_string()
    });

    // Parse user/password/database from URL
    // Expected format: http://user:password@host:port/database
    let (user, password, database, base_url) = parse_clickhouse_url(&url);

    clickhouse::Client::default()
        .with_url(&base_url)
        .with_user(&user)
        .with_password(&password)
        .with_database(&database)
        // Bound every query the app sends. On a 1 GiB-capped ClickHouse (see
        // clickhouse/config.d/low-mem.xml) a single unbounded aggregation can
        // trip the server-wide OvercommitTracker, and a killed query can leak
        // the global memory counter. Cap per-query memory and spill GROUP BY
        // to disk instead of dying at the ceiling. These are harmless for the
        // sink's INSERTs (no aggregation, well under the cap).
        .with_option("max_threads", "2")
        .with_option("max_memory_usage", "300000000") // ~300 MiB / query
        .with_option("max_bytes_before_external_group_by", "150000000") // spill GROUP BY
}

fn parse_clickhouse_url(url: &str) -> (String, String, String, String) {
    // Default values
    let mut user = "default".to_string();
    let mut password = String::new();
    let mut database = "default".to_string();

    // Strip scheme
    let rest = if let Some(s) = url.strip_prefix("http://") {
        s
    } else if let Some(s) = url.strip_prefix("https://") {
        s
    } else {
        url
    };

    let scheme = if url.starts_with("https://") {
        "https"
    } else {
        "http"
    };

    // Split userinfo@host/db
    let (userinfo_host, db_part) = if let Some(slash) = rest.find('/') {
        let db = &rest[slash + 1..];
        if !db.is_empty() {
            database = db.to_string();
        }
        (&rest[..slash], db.to_string())
    } else {
        (rest, database.clone())
    };
    let _ = db_part;

    let (userinfo, host) = if let Some(at) = userinfo_host.rfind('@') {
        (&userinfo_host[..at], &userinfo_host[at + 1..])
    } else {
        ("", userinfo_host)
    };

    if !userinfo.is_empty() {
        if let Some(colon) = userinfo.find(':') {
            user = userinfo[..colon].to_string();
            password = userinfo[colon + 1..].to_string();
        } else {
            user = userinfo.to_string();
        }
    }

    let base_url = format!("{}://{}", scheme, host);
    (user, password, database, base_url)
}

pub async fn run_migrations(client: &clickhouse::Client) {
    let create_migrations_table = "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            filename String,
            applied_at DateTime DEFAULT now()
        ) ENGINE = MergeTree ORDER BY filename
    ";

    if let Err(e) = client.query(create_migrations_table).execute().await {
        log::error!("ClickHouse: failed to create schema_migrations table: {e}");
        return;
    }

    log::info!(
        "ClickHouse: checking {} embedded migration(s)",
        CLICKHOUSE_MIGRATIONS.len()
    );

    let mut applied = 0usize;
    let mut skipped = 0usize;
    let mut errored = 0usize;
    let total = CLICKHOUSE_MIGRATIONS.len();

    for (idx, (filename, sql)) in CLICKHOUSE_MIGRATIONS.iter().enumerate() {
        let check = format!(
            "SELECT count() FROM schema_migrations WHERE filename = '{}'",
            filename.replace('\'', "\\'")
        );

        let count: u64 = match client.query(&check).fetch_one().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("ClickHouse: failed to check migration {filename}: {e}");
                errored += 1;
                continue;
            }
        };

        if count > 0 {
            log::info!(
                "ClickHouse migration [{}/{total}] already applied: {filename}",
                idx + 1
            );
            skipped += 1;
            continue;
        }

        log::info!(
            "ClickHouse migration [{}/{total}] applying: {filename}",
            idx + 1
        );

        if let Err(e) = client.query(sql).execute().await {
            log::error!("ClickHouse: failed to apply migration {filename}: {e}");
            errored += 1;
            continue;
        }

        let insert_sql = format!(
            "INSERT INTO schema_migrations (filename) VALUES ('{}')",
            filename.replace('\'', "\\'")
        );
        if let Err(e) = client.query(&insert_sql).execute().await {
            // DDL ran but bookkeeping failed: the migration will re-run on the
            // next startup. Count it as errored, not applied, so the summary
            // reflects that the schema_migrations record is missing.
            log::error!("ClickHouse: failed to record migration {filename}: {e}");
            errored += 1;
        } else {
            log::info!(
                "ClickHouse migration [{}/{total}] applied: {filename}",
                idx + 1
            );
            applied += 1;
        }
    }

    log::info!(
        "ClickHouse migrations done: {applied} newly applied, {skipped} already present, {errored} errored ({total} total)"
    );
}
