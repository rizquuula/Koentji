use std::fs;
use std::path::Path;

fn main() {
    let migrations_dir = Path::new("migrations");

    let mut files: Vec<_> = fs::read_dir(migrations_dir)
        .expect("Failed to read migrations directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sql"))
        .collect();
    files.sort();

    let mut code = String::from("pub static MIGRATIONS: &[(&str, &str)] = &[\n");
    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy();
        // Use forward slashes for include_str! path — works on all platforms
        let rel_path = path.to_string_lossy().replace('\\', "/");
        code.push_str(&format!(
            "    ({:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}\"))),\n",
            filename.as_ref(),
            rel_path,
        ));
    }
    code.push_str("];\n");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    fs::write(format!("{out_dir}/migrations.rs"), code).unwrap();

    // Re-run build script if any migration file changes
    println!("cargo:rerun-if-changed=migrations/");

    // ClickHouse migrations — optional dir
    let ch_migrations_dir = Path::new("clickhouse/migrations");
    let mut ch_code = String::from("pub static CLICKHOUSE_MIGRATIONS: &[(&str, &str)] = &[\n");
    if ch_migrations_dir.exists() {
        let mut ch_files: Vec<_> = fs::read_dir(ch_migrations_dir)
            .expect("Failed to read clickhouse/migrations directory")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sql"))
            .collect();
        ch_files.sort();
        for path in &ch_files {
            let filename = path.file_name().unwrap().to_string_lossy();
            let rel_path = path.to_string_lossy().replace('\\', "/");
            ch_code.push_str(&format!(
                "    ({:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}\"))),\n",
                filename.as_ref(),
                rel_path,
            ));
        }
    }
    ch_code.push_str("];\n");
    fs::write(format!("{out_dir}/clickhouse_migrations.rs"), ch_code).unwrap();
    println!("cargo:rerun-if-changed=clickhouse/migrations/");
}
