use crate::components::layout::Layout;
use leptos::prelude::*;

#[component]
pub fn TermsPage() -> impl IntoView {
    view! {
        <Layout active_tab="terms" require_auth=false>
            <div class="max-w-3xl mx-auto">
                <h1 class="text-3xl font-bold text-gray-900 mb-2">"Terms of Service"</h1>
                <p class="text-gray-500 mb-8">"Please read these terms carefully before using Koentji."</p>

                <div class="bg-white rounded-lg shadow p-6 space-y-6">
                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"1. Acceptance of Terms"</h2>
                        <p class="text-gray-600 text-sm">
                            "By accessing or using Koentji, you agree to be bound by these Terms of Service. "
                            "If you do not agree, do not use the service."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"2. Service Description"</h2>
                        <p class="text-gray-600 text-sm">
                            "Koentji provides API key management and authentication services. "
                            "Administrators can issue, manage, and revoke API keys with subscription tiers and rate limits. "
                            "External applications authenticate by calling the /v1/auth endpoint with an API key and device ID."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"3. API Key Usage"</h2>
                        <p class="text-gray-600 text-sm">
                            "API keys issued through Koentji are bound to specific devices and subscription tiers. "
                            "You are responsible for keeping your API keys secure. Do not share keys publicly or embed them in client-side code. "
                            "Each key has a rate limit determined by its subscription tier, and exceeding the limit will result in request rejection (HTTP 429)."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"4. Free Trial Keys"</h2>
                        <p class="text-gray-600 text-sm">
                            "Free trial keys are automatically created for new devices and expire on the 1st of the following month. "
                            "Free trial keys are subject to the rate limits defined for the free trial subscription tier."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"5. Rate Limits"</h2>
                        <p class="text-gray-600 text-sm">
                            "Rate limits are enforced per API key based on the assigned subscription tier and rate limit interval. "
                            "Rate limits reset automatically according to the configured interval. "
                            "Requests that exceed the rate limit will receive a 429 Too Many Requests response."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"6. Key Revocation"</h2>
                        <p class="text-gray-600 text-sm">
                            "Administrators may deactivate or delete API keys at any time. "
                            "Deactivated or deleted keys will immediately return 401 Unauthorized on authentication attempts."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"7. Disclaimer"</h2>
                        <p class="text-gray-600 text-sm">
                            "Koentji is provided \"as is\" without warranty of any kind. "
                            "As an open-source project, it is offered without guarantees of availability, accuracy, or fitness for a particular purpose."
                        </p>
                    </section>

                    <section>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"8. Changes to Terms"</h2>
                        <p class="text-gray-600 text-sm">
                            "These terms may be updated from time to time. Continued use of the service after changes constitutes acceptance of the updated terms."
                        </p>
                    </section>

                    <section class="pt-4 border-t">
                        <p class="text-gray-500 text-sm">
                            "Questions about these terms? Open an issue on "
                            <a href="https://github.com/rizquuula/Koentji" target="_blank" rel="noopener noreferrer" class="text-blue-600 hover:text-blue-800">"GitHub"</a>
                            "."
                        </p>
                    </section>
                </div>
            </div>
        </Layout>
    }
}
