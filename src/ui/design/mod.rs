//! Design primitives.
//!
//! Six building blocks replace the class-soup copy-paste across forms and
//! pages: `Button`, `Input`, `Select`, `Surface`, `Stack`, `Badge`. They
//! compose on top of the semantic tokens introduced in 6.1
//! (`brand`/`surface`/`ink`/`feedback` + `rounded-control`/`rounded-card`).

pub mod badge;
pub mod button;
pub mod data_table;
pub mod input;
pub mod modal;
pub mod page_header;
pub mod select;
pub mod stack;
pub mod surface;
pub mod toast;

pub use badge::{Badge, BadgeTone};
pub use button::{Button, ButtonType, ButtonVariant};
pub use data_table::DataTable;
pub use input::Input;
pub use page_header::PageHeader;
pub use select::Select;
pub use stack::{Stack, StackGap};
pub use surface::Surface;
