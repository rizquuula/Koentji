// Dashboard chart renderer. The Rust side calls the exported
// `renderCharts(dataJson)` global via a typed `#[wasm_bindgen]` extern
// — no `eval`, no `window.__chartData` smuggling.
//
// The argument is a JSON *string* so the wasm → JS crossing stays a
// single primitive value. The body defers via setTimeout so canvas
// elements are live in the DOM before Chart.js probes them.

let subscriptionChart = null;
let rateLimitChart = null;
let trendChart = null;

const CHART_COLORS = [
    '#3B82F6', '#10B981', '#F59E0B', '#EF4444',
    '#8B5CF6', '#EC4899', '#14B8A6', '#F97316'
];

function renderCharts(dataJson) {
    setTimeout(function() {
        if (typeof dataJson !== 'string' || dataJson.length === 0) return;
        let data;
        try {
            data = JSON.parse(dataJson);
        } catch (_) {
            return;
        }
        renderChartsImpl(data);
    }, 0);
}

function renderChartsImpl(data) {
    // Subscription Pie Chart
    const subCtx = document.getElementById('subscription-chart');
    if (subCtx) {
        if (subscriptionChart) subscriptionChart.destroy();
        subscriptionChart = new Chart(subCtx, {
            type: 'pie',
            data: {
                labels: data.subscriptionLabels,
                datasets: [{
                    data: data.subscriptionValues,
                    backgroundColor: CHART_COLORS.slice(0, data.subscriptionLabels.length),
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: { position: 'bottom', labels: { boxWidth: 12, padding: 10 } }
                }
            }
        });
    }

    // Rate Limit Bar Chart
    const rateCtx = document.getElementById('rate-limit-chart');
    if (rateCtx) {
        if (rateLimitChart) rateLimitChart.destroy();
        rateLimitChart = new Chart(rateCtx, {
            type: 'bar',
            data: {
                labels: data.rateLimitLabels,
                datasets: [{
                    label: 'Keys',
                    data: data.rateLimitValues,
                    backgroundColor: ['#10B981', '#F59E0B', '#F97316', '#EF4444'],
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { display: false } },
                scales: {
                    y: { beginAtZero: true, ticks: { stepSize: 1 } }
                }
            }
        });
    }

    // Daily Trend Line Chart
    const trendCtx = document.getElementById('trend-chart');
    if (trendCtx) {
        if (trendChart) trendChart.destroy();
        trendChart = new Chart(trendCtx, {
            type: 'line',
            data: {
                labels: data.trendLabels,
                datasets: [{
                    label: 'Keys Created',
                    data: data.trendValues,
                    borderColor: '#3B82F6',
                    backgroundColor: 'rgba(59, 130, 246, 0.1)',
                    fill: true,
                    tension: 0.3,
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { display: false } },
                scales: {
                    y: { beginAtZero: true, ticks: { stepSize: 1 } }
                }
            }
        });
    }
}
