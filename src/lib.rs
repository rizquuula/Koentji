pub mod app;
pub mod auth;
#[cfg(feature = "ssr")]
pub mod cache;
pub mod components;
#[cfg(feature = "ssr")]
pub mod db;
pub mod error;
pub mod models;
pub mod pages;
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
