#![recursion_limit = "256"]
pub mod app;
#[cfg(feature = "ssr")]
pub mod application;
pub mod auth;
pub mod components;
#[cfg(feature = "ssr")]
pub mod db;
pub mod domain;
pub mod error;
#[cfg(feature = "ssr")]
pub mod infrastructure;
pub mod interface;
pub mod models;
pub mod pages;
#[cfg(feature = "ssr")]
pub mod rate_limit;
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
