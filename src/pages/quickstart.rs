use crate::components::layout::Layout;
use leptos::prelude::*;

#[component]
pub fn QuickstartPage() -> impl IntoView {
    view! {
        <Layout active_tab="quickstart" require_auth=false>
            <div class="max-w-3xl space-y-10">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900">"Quickstart"</h1>
                    <p class="mt-2 text-gray-500">
                        "Learn how to validate API keys from your application using Koentji."
                    </p>
                </div>

                // Step 1
                <section class="space-y-3">
                    <div class="flex items-center space-x-3">
                        <span class="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 text-white text-sm font-bold flex items-center justify-center">"1"</span>
                        <h2 class="text-xl font-semibold text-gray-900">"Create an API Key"</h2>
                    </div>
                    <p class="text-gray-600 pl-10">
                        "Go to the "
                        <a href="/keys" class="text-blue-600 hover:underline">"API Keys"</a>
                        " page and click " <strong>"\"Create Key\""</strong>
                        ". Fill in the device ID, subscription tier, and optional expiry. You will receive a key prefixed with "
                        <code class="bg-gray-100 px-1 rounded text-sm">"klab_"</code>"."
                    </p>
                </section>

                // Step 2
                <section class="space-y-3">
                    <div class="flex items-center space-x-3">
                        <span class="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 text-white text-sm font-bold flex items-center justify-center">"2"</span>
                        <h2 class="text-xl font-semibold text-gray-900">"Validate the Key"</h2>
                    </div>
                    <p class="text-gray-600 pl-10">
                        "Send a " <code class="bg-gray-100 px-1 rounded text-sm">"GET"</code>
                        " request to the validate endpoint with the key in the "
                        <code class="bg-gray-100 px-1 rounded text-sm">"X-API-Key"</code>" header."
                    </p>

                    <div class="pl-10 space-y-4">
                        // Request example
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Request"</span>
                                <span class="text-xs text-gray-500">"curl"</span>
                            </div>
                            <pre class="bg-gray-900 text-green-400 text-sm p-4 overflow-x-auto"><code>
"curl -X GET https://your-domain.com/api/validate \\\n  -H \"X-API-Key: klab_your_api_key_here\""
                            </code></pre>
                        </div>

                        // Success response
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — valid key"</span>
                                <span class="text-xs text-green-400">"200 OK"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"{\n  \"valid\": true,\n  \"subscription\": \"pro\",\n  \"rate_limit_daily\": 1000,\n  \"rate_limit_remaining\": 842\n}"
                            </code></pre>
                        </div>
                    </div>
                </section>

                // Step 3
                <section class="space-y-3">
                    <div class="flex items-center space-x-3">
                        <span class="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 text-white text-sm font-bold flex items-center justify-center">"3"</span>
                        <h2 class="text-xl font-semibold text-gray-900">"Handle Error Responses"</h2>
                    </div>
                    <p class="text-gray-600 pl-10">
                        "If the key is missing, invalid, expired, or rate-limited, Koentji returns a non-200 status with a JSON error body."
                    </p>

                    <div class="pl-10 space-y-4">
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — missing key"</span>
                                <span class="text-xs text-red-400">"401 Unauthorized"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"{ \"error\": \"missing_api_key\", \"message\": \"X-API-Key header is required\" }"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — invalid or deleted key"</span>
                                <span class="text-xs text-red-400">"401 Unauthorized"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"{ \"error\": \"invalid_api_key\", \"message\": \"The provided API key is not valid\" }"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — expired key"</span>
                                <span class="text-xs text-red-400">"401 Unauthorized"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"{ \"error\": \"key_expired\", \"message\": \"This API key has expired\" }"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — rate limit exceeded"</span>
                                <span class="text-xs text-yellow-400">"429 Too Many Requests"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"{ \"error\": \"rate_limit_exceeded\", \"message\": \"Daily rate limit reached\", \"rate_limit_remaining\": 0 }"
                            </code></pre>
                        </div>
                    </div>
                </section>

                // Step 4
                <section class="space-y-3">
                    <div class="flex items-center space-x-3">
                        <span class="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 text-white text-sm font-bold flex items-center justify-center">"4"</span>
                        <h2 class="text-xl font-semibold text-gray-900">"Example Integration"</h2>
                    </div>
                    <p class="text-gray-600 pl-10">
                        "A minimal middleware example in any HTTP server — check the key before processing the request."
                    </p>

                    <div class="pl-10 space-y-4">
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"JavaScript / Node.js"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"async function validateKey(apiKey) {\n  const res = await fetch('https://your-domain.com/api/validate', {\n    headers: { 'X-API-Key': apiKey },\n  });\n  if (!res.ok) {\n    const err = await res.json();\n    throw new Error(err.message);\n  }\n  return res.json(); // { valid, subscription, rate_limit_remaining, ... }\n}"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Python"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
"import requests\n\ndef validate_key(api_key: str) -> dict:\n    response = requests.get(\n        'https://your-domain.com/api/validate',\n        headers={'X-API-Key': api_key},\n    )\n    response.raise_for_status()  # raises on 4xx/5xx\n    return response.json()"
                            </code></pre>
                        </div>
                    </div>
                </section>

                // Subscription tiers table
                <section class="space-y-3">
                    <h2 class="text-xl font-semibold text-gray-900">"Subscription Tiers"</h2>
                    <div class="overflow-hidden border border-gray-200 rounded-lg">
                        <table class="min-w-full divide-y divide-gray-200 text-sm">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Tier"</th>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Daily Limit"</th>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Notes"</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-gray-200 bg-white">
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"free"</td>
                                    <td class="px-4 py-3 text-gray-600">"100 req / day"</td>
                                    <td class="px-4 py-3 text-gray-500">"Default tier"</td>
                                </tr>
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"basic"</td>
                                    <td class="px-4 py-3 text-gray-600">"500 req / day"</td>
                                    <td class="px-4 py-3 text-gray-500">""</td>
                                </tr>
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"pro"</td>
                                    <td class="px-4 py-3 text-gray-600">"1 000 req / day"</td>
                                    <td class="px-4 py-3 text-gray-500">""</td>
                                </tr>
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"enterprise"</td>
                                    <td class="px-4 py-3 text-gray-600">"Unlimited"</td>
                                    <td class="px-4 py-3 text-gray-500">"Custom rate limit"</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </section>
            </div>
        </Layout>
    }
}
