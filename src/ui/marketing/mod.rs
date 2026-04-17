//! Marketing bounded context — public pages: landing, about, terms, privacy,
//! and the quickstart guide for API integrators.

pub mod about;
pub mod landing;
pub mod privacy;
pub mod quickstart;
pub mod terms;

pub use about::AboutPage;
pub use landing::LandingPage;
pub use privacy::PrivacyPage;
pub use quickstart::QuickstartPage;
pub use terms::TermsPage;
