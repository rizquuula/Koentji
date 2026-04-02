use crate::components::toast::provide_toast_context;
use crate::pages::{
    dashboard::DashboardPage, keys::KeysPage, login::LoginPage, quickstart::QuickstartPage,
    rate_limits::LimitsIntervalPage, subscriptions::SubscriptionsPage,
};
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

    view! {
        <Stylesheet id="leptos" href="/pkg/koentji.css"/>
        <Title text="Koentji"/>
        <Script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"/>
        <Script src="/assets/js/charts.js"/>

        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/login") view=LoginPage/>
                <Route path=path!("/dashboard") view=DashboardPage/>
                <Route path=path!("/keys") view=KeysPage/>
                <Route path=path!("/subscriptions") view=SubscriptionsPage/>
                <Route path=path!("/limits-interval") view=LimitsIntervalPage/>
                <Route path=path!("/quickstart") view=QuickstartPage/>
                <Route path=path!("/") view=|| view! { <leptos_router::components::Redirect path="/dashboard"/> }/>
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
                <a href="/dashboard" class="mt-4 inline-block text-blue-600 hover:text-blue-800">"Go to Dashboard"</a>
            </div>
        </div>
    }
}
