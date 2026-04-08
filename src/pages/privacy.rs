use crate::components::layout::Layout;
use leptos::prelude::*;

#[component]
pub fn PrivacyPage() -> impl IntoView {
    view! {
        <Layout active_tab="privacy" require_auth=false>
            <div class="max-w-3xl mx-auto">
                <h1 class="text-3xl font-bold text-gray-900 mb-2">"Privacy Policy"</h1>
                <p class="text-gray-500 mb-8">"How Koentji handles your data."</p>

                <div class="bg-white rounded-lg shadow p-6 space-y-6">
                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"1. Data Collected"</h2>
                        <p class="text-gray-600 text-sm mb-2">
                            "Koentji collects and stores the following data as part of its API key management functionality:"
                        </p>
                        <ul class="list-disc list-inside text-gray-600 text-sm space-y-1 ml-2">
                            <li>"API keys and their associated metadata (name, status, expiry date)"</li>
                            <li>"Device IDs provided during authentication requests"</li>
                            <li>"Rate limit usage counts per API key"</li>
                            <li>"Subscription tier assignments"</li>
                            <li>"Authentication request timestamps"</li>
                        </ul>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"2. How Data Is Used"</h2>
                        <p class="text-gray-600 text-sm">
                            "Data is used solely for API key authentication and rate limit enforcement. "
                            "When a request is made to /v1/auth, Koentji validates the API key, verifies the device ID, "
                            "checks subscription status and expiry, and enforces rate limits based on the assigned tier and interval."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"3. Data Storage"</h2>
                        <p class="text-gray-600 text-sm">
                            "All data is stored in a PostgreSQL database configured by the service operator. "
                            "An in-memory cache (with a configurable TTL, default 15 minutes) is used to speed up authentication lookups. "
                            "Data storage location and security depend on the operator's deployment configuration."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"4. Sessions & Cookies"</h2>
                        <p class="text-gray-600 text-sm">
                            "The admin dashboard uses HTTP-only encrypted session cookies for authentication. "
                            "Sessions expire after 24 hours. No tracking cookies or third-party analytics are used."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"5. Third-Party Sharing"</h2>
                        <p class="text-gray-600 text-sm">
                            "Koentji does not share any data with third parties. "
                            "All data remains within the service operator's infrastructure."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"6. Data Retention"</h2>
                        <p class="text-gray-600 text-sm">
                            "API key records and usage data are retained as long as the key exists. "
                            "Deleted keys are removed from the database. "
                            "Free trial keys expire on the 1st of the month following creation."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"7. Self-Hosted"</h2>
                        <p class="text-gray-600 text-sm">
                            "Koentji is a self-hosted open-source application. The service operator is responsible for "
                            "data security, backups, and compliance with applicable data protection regulations."
                        </p>
                    </section>

                    <section class="pt-4 border-t">
                        <p class="text-gray-500 text-sm">
                            "Questions about this policy? Open an issue on "
                            <a href="https://github.com/rizquuula/Koentji" target="_blank" rel="noopener noreferrer" class="text-blue-600 hover:text-blue-800">"GitHub"</a>
                            "."
                        </p>
                    </section>
                </div>
            </div>
        </Layout>
    }
}
