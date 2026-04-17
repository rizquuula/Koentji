//! `hash-admin-password` — print an argon2id PHC hash for
//! `ADMIN_PASSWORD_HASH`.
//!
//! Usage:
//!
//! ```text
//!   cargo run --features ssr --bin hash-admin-password -- "mypassword"
//!   make hash-admin-password PASSWORD=mypassword
//! ```
//!
//! Reads the plaintext from argv[1] (or from stdin if no argument is
//! given) and prints the PHC string to stdout. Paste the output into
//! the `ADMIN_PASSWORD_HASH` env var and remove `ADMIN_PASSWORD` — the
//! dashboard prefers the hash and will log a warning if the plaintext
//! fallback is in use.

#[cfg(feature = "ssr")]
fn main() {
    use std::io::{self, BufRead, Write};

    let args: Vec<String> = std::env::args().collect();
    let plaintext = if let Some(pw) = args.get(1) {
        pw.clone()
    } else {
        eprint!("Password: ");
        io::stderr().flush().ok();
        let mut buf = String::new();
        io::stdin()
            .lock()
            .read_line(&mut buf)
            .expect("failed to read password from stdin");
        buf.trim_end_matches(['\n', '\r']).to_string()
    };

    if plaintext.is_empty() {
        eprintln!("error: password is empty");
        std::process::exit(1);
    }

    match koentji::infrastructure::hashing::hash_password(&plaintext) {
        Ok(phc) => {
            println!("{phc}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    eprintln!("hash-admin-password requires the `ssr` feature");
    std::process::exit(1);
}
