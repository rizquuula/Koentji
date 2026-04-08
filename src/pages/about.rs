use crate::components::layout::Layout;
use leptos::prelude::*;

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <Layout active_tab="about" require_auth=false>
            <div class="max-w-3xl mx-auto">
                <h1 class="text-3xl font-bold text-gray-900 mb-2">"About Koentji"</h1>
                <p class="text-gray-500 mb-8">"An open-source API key management and authentication service."</p>

                // What is Koentji
                <section class="bg-white rounded-lg shadow p-6 mb-6">
                    <h2 class="text-xl font-semibold text-gray-900 mb-3">"What is Koentji?"</h2>
                    <p class="text-gray-600 mb-3">
                        "Koentji is a self-hosted service for issuing, managing, and revoking API keys. "
                        "It provides an admin dashboard where you can create API keys bound to specific devices, "
                        "assign subscription tiers with configurable rate limits, and monitor usage."
                    </p>
                    <p class="text-gray-600">
                        "External applications authenticate their users by calling the "
                        <code class="bg-gray-100 px-1.5 py-0.5 rounded text-sm font-mono">"POST /v1/auth"</code>
                        " endpoint with an API key and device ID. Koentji validates the key, checks rate limits, "
                        "and returns the authentication result with remaining quota information."
                    </p>
                </section>

                // How It Works
                <section class="bg-white rounded-lg shadow p-6 mb-6">
                    <h2 class="text-xl font-semibold text-gray-900 mb-3">"How It Works"</h2>
                    <div class="space-y-4">
                        <div class="flex items-start space-x-3">
                            <span class="flex-shrink-0 w-7 h-7 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center text-sm font-semibold">"1"</span>
                            <div>
                                <p class="font-medium text-gray-900">"Create API Keys"</p>
                                <p class="text-gray-500 text-sm">"Generate keys (prefixed with klab_) through the admin dashboard. Each key is bound to a device ID and assigned a subscription tier."</p>
                            </div>
                        </div>
                        <div class="flex items-start space-x-3">
                            <span class="flex-shrink-0 w-7 h-7 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center text-sm font-semibold">"2"</span>
                            <div>
                                <p class="font-medium text-gray-900">"Authenticate Requests"</p>
                                <p class="text-gray-500 text-sm">"Your application sends the API key and device ID to POST /v1/auth. Koentji validates the key, checks expiry, and enforces rate limits."</p>
                            </div>
                        </div>
                        <div class="flex items-start space-x-3">
                            <span class="flex-shrink-0 w-7 h-7 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center text-sm font-semibold">"3"</span>
                            <div>
                                <p class="font-medium text-gray-900">"Manage Subscriptions & Limits"</p>
                                <p class="text-gray-500 text-sm">"Configure subscription tiers with different rate limits. Set rate limit intervals that reset automatically. Support free trial keys with automatic expiry."</p>
                            </div>
                        </div>
                    </div>
                </section>

                // Tech Stack
                <section class="bg-white rounded-lg shadow p-6 mb-6">
                    <h2 class="text-xl font-semibold text-gray-900 mb-3">"Tech Stack"</h2>
                    <div class="grid grid-cols-2 sm:grid-cols-4 gap-4">
                        <div class="text-center p-3 bg-gray-50 rounded-lg">
                            <p class="font-semibold text-gray-900">"Rust"</p>
                            <p class="text-gray-500 text-xs">"Backend & WASM"</p>
                        </div>
                        <div class="text-center p-3 bg-gray-50 rounded-lg">
                            <p class="font-semibold text-gray-900">"Leptos"</p>
                            <p class="text-gray-500 text-xs">"SSR + Hydration"</p>
                        </div>
                        <div class="text-center p-3 bg-gray-50 rounded-lg">
                            <p class="font-semibold text-gray-900">"Actix-Web"</p>
                            <p class="text-gray-500 text-xs">"HTTP Server"</p>
                        </div>
                        <div class="text-center p-3 bg-gray-50 rounded-lg">
                            <p class="font-semibold text-gray-900">"PostgreSQL"</p>
                            <p class="text-gray-500 text-xs">"Database"</p>
                        </div>
                    </div>
                </section>

                // Open Source
                <section class="bg-white rounded-lg shadow p-6">
                    <h2 class="text-xl font-semibold text-gray-900 mb-3">"Open Source"</h2>
                    <p class="text-gray-600 mb-4">
                        "Koentji is open source and available on GitHub. "
                        "You can view the source code, report issues, or contribute to the project."
                    </p>
                    <a
                        href="https://github.com/rizquuula/Koentji"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="inline-flex items-center space-x-2 px-4 py-2 bg-gray-900 text-white rounded-lg hover:bg-gray-800 transition-colors text-sm"
                    >
                        <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
                        </svg>
                        <span>"View on GitHub"</span>
                    </a>
                </section>
            </div>
        </Layout>
    }
}
