use leptos::prelude::*;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50">
            // Hero Section
            <div class="bg-white">
                <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-20 text-center">
                    <div class="flex items-center justify-center space-x-3 mb-6">
                        <svg class="w-14 h-14 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                        </svg>
                        <h1 class="text-4xl sm:text-5xl font-bold text-gray-900">"Koentji"</h1>
                    </div>
                    <p class="text-xl text-gray-600 mb-4">"API Key Management & Authentication Service"</p>
                    <p class="text-gray-500 max-w-2xl mx-auto mb-10">
                        "Issue, manage, and revoke API keys with subscription tiers and rate limits. "
                        "Authenticate external applications via a single endpoint."
                    </p>
                    <div class="flex items-center justify-center space-x-4">
                        <a
                            href="/login"
                            class="px-8 py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors"
                        >
                            "Sign In"
                        </a>
                        <a
                            href="/quickstart"
                            class="px-8 py-3 bg-white text-blue-600 font-medium rounded-lg border border-blue-600 hover:bg-blue-50 transition-colors"
                        >
                            "View Quickstart"
                        </a>
                    </div>
                </div>
            </div>

            // Features Section
            <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
                <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                    // API Key Management
                    <div class="bg-white rounded-lg shadow p-6">
                        <div class="w-10 h-10 bg-blue-50 rounded-lg flex items-center justify-center mb-4">
                            <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                            </svg>
                        </div>
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"API Key Management"</h3>
                        <p class="text-gray-500 text-sm">
                            "Create, activate, deactivate, and delete API keys bound to specific devices. "
                            "Each key is prefixed with klab_ and tied to a subscription tier."
                        </p>
                    </div>

                    // Rate Limiting
                    <div class="bg-white rounded-lg shadow p-6">
                        <div class="w-10 h-10 bg-green-50 rounded-lg flex items-center justify-center mb-4">
                            <svg class="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                        </div>
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"Rate Limiting"</h3>
                        <p class="text-gray-500 text-sm">
                            "Configurable rate limit intervals that reset automatically. "
                            "Track remaining usage per key and respond with 429 when limits are exceeded."
                        </p>
                    </div>

                    // Subscription Tiers
                    <div class="bg-white rounded-lg shadow p-6">
                        <div class="w-10 h-10 bg-purple-50 rounded-lg flex items-center justify-center mb-4">
                            <svg class="w-6 h-6 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z"/>
                            </svg>
                        </div>
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"Subscription Tiers"</h3>
                        <p class="text-gray-500 text-sm">
                            "Define subscription types with different rate limits. "
                            "Assign tiers to API keys and support free trial keys that expire automatically."
                        </p>
                    </div>
                </div>
            </div>

            // Open Source Banner
            <div class="bg-gray-900 text-white">
                <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-12 text-center">
                    <h2 class="text-2xl font-bold mb-3">"Open Source"</h2>
                    <p class="text-gray-300 mb-6 max-w-xl mx-auto">
                        "Koentji is open source. View the code, report issues, or contribute on GitHub."
                    </p>
                    <a
                        href="https://github.com/rizquuula/Koentji"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="inline-flex items-center space-x-2 px-6 py-3 bg-white text-gray-900 font-medium rounded-lg hover:bg-gray-100 transition-colors"
                    >
                        <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
                        </svg>
                        <span>"View on GitHub"</span>
                    </a>
                </div>
            </div>

            // Footer
            <footer class="bg-white border-t">
                <div class="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                    <div class="flex flex-col sm:flex-row items-center justify-between space-y-4 sm:space-y-0">
                        <div class="flex items-center space-x-2 text-gray-500 text-sm">
                            <svg class="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                            </svg>
                            <span>"Koentji"</span>
                        </div>
                        <div class="flex items-center space-x-6 text-sm">
                            <a href="/about" class="text-gray-500 hover:text-gray-700">"About"</a>
                            <a href="/quickstart" class="text-gray-500 hover:text-gray-700">"Quickstart"</a>
                            <a href="/docs" target="_blank" class="text-gray-500 hover:text-gray-700">"API Docs"</a>
                            <a href="/terms" class="text-gray-500 hover:text-gray-700">"Terms"</a>
                            <a href="/privacy" class="text-gray-500 hover:text-gray-700">"Privacy"</a>
                            <a href="https://github.com/rizquuula/Koentji" target="_blank" rel="noopener noreferrer" class="text-gray-500 hover:text-gray-700">"GitHub"</a>
                        </div>
                    </div>
                </div>
            </footer>
        </div>
    }
}
