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
}
