use crate::ui::shell::layout::Layout;
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
                        "Send a " <code class="bg-gray-100 px-1 rounded text-sm">"POST"</code>
                        " request to " <code class="bg-gray-100 px-1 rounded text-sm">"/v1/auth"</code>
                        " with a JSON body containing the key, device ID, and optional usage units."
                    </p>

                    <div class="pl-10 space-y-4">
                        // Request example
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Request"</span>
                                <span class="text-xs text-gray-500">"curl"</span>
                            </div>
                            <pre class="bg-gray-900 text-green-400 text-sm p-4 overflow-x-auto"><code>
    "curl -X POST https://your-domain.com/v1/auth \\\n  -H \"Content-Type: application/json\" \\\n  -d '{\n    \"auth_key\": \"klab_your_api_key_here\",\n    \"auth_device\": \"device_id_here\",\n    \"rate_limit_usage\": 1\n  }'"
                            </code></pre>
                        </div>

                        // Success response
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — valid key"</span>
                                <span class="text-xs text-green-400">"200 OK"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
    "{\n  \"status\": \"success\",\n  \"data\": {\n    \"key\": \"klab_your_api_key_here\",\n    \"device\": \"device_id_here\",\n    \"subscription\": \"pro\",\n    \"username\": \"john\",\n    \"email\": \"john@example.com\",\n    \"valid_until\": \"2026-12-31T00:00:00Z\",\n    \"rate_limit_remaining\": 842\n  }\n}"
                            </code></pre>
                        </div>
                    </div>
                </section>

                // Request fields table
                <section class="space-y-3 pl-10">
                    <h3 class="text-base font-semibold text-gray-700">"Request Fields"</h3>
                    <div class="overflow-hidden border border-gray-200 rounded-lg">
                        <table class="min-w-full divide-y divide-gray-200 text-sm">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Field"</th>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Type"</th>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Required"</th>
                                    <th class="px-4 py-3 text-left font-medium text-gray-500">"Description"</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-gray-200 bg-white">
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"auth_key"</td>
                                    <td class="px-4 py-3 text-gray-600">"string"</td>
                                    <td class="px-4 py-3 text-gray-600">"Yes"</td>
                                    <td class="px-4 py-3 text-gray-500">"The API key to authenticate"</td>
                                </tr>
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"auth_device"</td>
                                    <td class="px-4 py-3 text-gray-600">"string"</td>
                                    <td class="px-4 py-3 text-gray-600">"Yes"</td>
                                    <td class="px-4 py-3 text-gray-500">"Device ID the key is bound to"</td>
                                </tr>
                                <tr>
                                    <td class="px-4 py-3 font-mono text-gray-800">"rate_limit_usage"</td>
                                    <td class="px-4 py-3 text-gray-600">"integer"</td>
                                    <td class="px-4 py-3 text-gray-600">"No (default: 1)"</td>
                                    <td class="px-4 py-3 text-gray-500">"Units to consume from the rate limit"</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </section>

                // Step 3
                <section class="space-y-3">
                    <div class="flex items-center space-x-3">
                        <span class="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 text-white text-sm font-bold flex items-center justify-center">"3"</span>
                        <h2 class="text-xl font-semibold text-gray-900">"Handle Error Responses"</h2>
                    </div>
                    <p class="text-gray-600 pl-10">
                        "If the key is invalid, expired, or rate-limited, Koentji returns a non-200 status with a JSON error body. "
                        "The " <code class="bg-gray-100 px-1 rounded text-sm">"error"</code>
                        " field contains localised messages (keys: " <code class="bg-gray-100 px-1 rounded text-sm">"en"</code>
                        ", " <code class="bg-gray-100 px-1 rounded text-sm">"id"</code>
                        ") and " <code class="bg-gray-100 px-1 rounded text-sm">"message"</code>
                        " is a plain-text fallback."
                    </p>

                    <div class="pl-10 space-y-4">
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — invalid or deleted key"</span>
                                <span class="text-xs text-red-400">"401 Unauthorized"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
    "{\n  \"error\": {\n    \"en\": \"Key not found or has been deleted.\",\n    \"id\": \"Kunci tidak ditemukan atau sudah dihapus.\"\n  },\n  \"message\": \"Key not found or has been deleted.\"\n}"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — expired key"</span>
                                <span class="text-xs text-red-400">"401 Unauthorized"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
    "{\n  \"error\": {\n    \"en\": \"This API key has expired.\",\n    \"id\": \"Kunci API ini sudah kedaluwarsa.\"\n  },\n  \"message\": \"This API key has expired.\"\n}"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Response — rate limit exceeded"</span>
                                <span class="text-xs text-yellow-400">"429 Too Many Requests"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
    "{\n  \"error\": {\n    \"en\": \"Rate limit exceeded. Please try again later or upgrade your subscription.\",\n    \"id\": \"Batas rate limit terlampaui. Silakan coba lagi nanti atau upgrade langganan Anda.\"\n  },\n  \"message\": \"Rate limit exceeded. Please try again later or upgrade your subscription.\"\n}"
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
                        "A minimal middleware example — call " <code class="bg-gray-100 px-1 rounded text-sm">"/v1/auth"</code>
                        " before processing each request."
                    </p>

                    <div class="pl-10 space-y-4">
                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"JavaScript / Node.js"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
    "async function validateKey(authKey, authDevice, usage = 1) {\n  const res = await fetch('https://your-domain.com/v1/auth', {\n    method: 'POST',\n    headers: { 'Content-Type': 'application/json' },\n    body: JSON.stringify({\n      auth_key: authKey,\n      auth_device: authDevice,\n      rate_limit_usage: usage,\n    }),\n  });\n  if (!res.ok) {\n    const err = await res.json();\n    throw new Error(err.message);\n  }\n  const { data } = await res.json();\n  return data; // { key, device, subscription, rate_limit_remaining, ... }\n}"
                            </code></pre>
                        </div>

                        <div class="rounded-lg overflow-hidden border border-gray-200">
                            <div class="bg-gray-800 px-4 py-2 flex items-center justify-between">
                                <span class="text-xs font-medium text-gray-400">"Python"</span>
                            </div>
                            <pre class="bg-gray-900 text-gray-300 text-sm p-4 overflow-x-auto"><code>
    "import requests\n\ndef validate_key(auth_key: str, auth_device: str, usage: int = 1) -> dict:\n    response = requests.post(\n        'https://your-domain.com/v1/auth',\n        json={\n            'auth_key': auth_key,\n            'auth_device': auth_device,\n            'rate_limit_usage': usage,\n        },\n    )\n    response.raise_for_status()  # raises on 4xx/5xx\n    return response.json()['data']"
                            </code></pre>
                        </div>
                    </div>
                </section>

                // Subscription tiers note
                <section class="space-y-3">
                    <h2 class="text-xl font-semibold text-gray-900">"Subscription Tiers & Rate Limits"</h2>
                    <p class="text-gray-600">
                        "Each subscription tier has a configurable rate limit interval and request cap. "
                        "The interval resets automatically — once the interval period has passed since the last request, the counter starts fresh. "
                        "Limits and intervals are managed by your administrator in the "
                        <a href="/subscriptions" class="text-blue-600 hover:underline">"Subscriptions"</a>
                        " and "
                        <a href="/rate-limits" class="text-blue-600 hover:underline">"Limits Interval"</a>
                        " pages."
                    </p>
                    <p class="text-gray-600">
                        "The " <code class="bg-gray-100 px-1 rounded text-sm">"rate_limit_remaining"</code>
                        " field in the response always reflects the up-to-date count after deducting "
                        <code class="bg-gray-100 px-1 rounded text-sm">"rate_limit_usage"</code>
                        " units from the current request."
                    </p>
                </section>
            </div>
        </Layout>
    }
}
