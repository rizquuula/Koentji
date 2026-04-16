//! Postgres-backed adapters for the domain's ports.

pub mod issued_key_repository;

pub use issued_key_repository::PostgresIssuedKeyRepository;
