use crate::ui::admin_access::LoginPage;
use crate::ui::analytics::AnalyticsPage;
use crate::ui::dashboard::DashboardPage;
use crate::ui::design::toast::provide_toast_context;
use crate::ui::keys::KeysPage;
use crate::ui::marketing::{AboutPage, LandingPage, PrivacyPage, QuickstartPage, TermsPage};
use crate::ui::rate_limits::LimitsIntervalPage;
use crate::ui::subscriptions::SubscriptionsPage;
use crate::ui::tz::provide_tz_offset;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_toast_context();
    provide_tz_offset();

    view! {
        // The hashed stylesheet <link> is emitted from the server shell
        // (src/main.rs) via <HashedStylesheet>, which needs LeptosOptions and
        // so can't live here. A hard-coded /pkg/koentji.css href would 404
        // once hash-files renames the file to koentji.<hash>.css.
        <Title text="Koentji"/>
        // Subresource Integrity: pin the exact chart.js bytes so a CDN
        // compromise or MITM can't inject arbitrary JS into the admin
        // origin. `crossorigin` is required for the browser to enforce
        // `integrity` on a cross-origin script.
        <Script
            src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"
            integrity="sha384-9nhczxUqK87bcKHh20fSQcTGD4qq5GhayNYSYWqwBkINBhOfQLg/P5HG5lF1urn4"
            crossorigin="anonymous"
        />
        <Script src="/assets/js/charts.js"/>
        <Script src="/assets/js/analytics_charts.js"/>

        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/login") view=LoginPage/>
                <Route path=path!("/dashboard") view=DashboardPage/>
                <Route path=path!("/analytics") view=AnalyticsPage/>
                <Route path=path!("/keys") view=KeysPage/>
                <Route path=path!("/subscriptions") view=SubscriptionsPage/>
                <Route path=path!("/limits-interval") view=LimitsIntervalPage/>
                <Route path=path!("/quickstart") view=QuickstartPage/>
                <Route path=path!("/about") view=AboutPage/>
                <Route path=path!("/terms") view=TermsPage/>
                <Route path=path!("/privacy") view=PrivacyPage/>
                <Route path=path!("/") view=LandingPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        let resp = leptos_actix::ResponseOptions::default();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <div class="min-h-screen flex items-center justify-center">
            <div class="text-center">
                <h1 class="text-6xl font-bold text-gray-300">"404"</h1>
                <p class="mt-4 text-gray-500">"Page not found"</p>
                <a href="/dashboard" rel="external" class="mt-4 inline-block text-blue-600 hover:text-blue-800">"Go to Dashboard"</a>
            </div>
        </div>
    }
}
