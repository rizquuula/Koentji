//! Analytics bounded context — admin-only ClickHouse-backed dashboards.

pub mod page;
pub mod panels;
pub mod summary_cards;
pub mod tables;

pub use page::AnalyticsPage;
