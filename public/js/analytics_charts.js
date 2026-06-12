// Analytics chart renderer. The Rust side calls the exported
// `renderAnalyticsCharts(dataJson)` global via a typed `#[wasm_bindgen]`
// extern — no `eval`, no `window.__chartData` smuggling.
//
// The argument is a JSON *string* so the wasm → JS crossing stays a
// single primitive value. The body defers via setTimeout so the canvas
// is live in the DOM before Chart.js probes it, and destroys the prior
// instance before re-creating so range switches don't leak canvases.

let trafficChart = null;

function renderAnalyticsCharts(dataJson) {
    setTimeout(function() {
        if (typeof dataJson !== 'string' || dataJson.length === 0) return;
        let data;
        try {
            data = JSON.parse(dataJson);
        } catch (_) {
            return;
        }
        renderAnalyticsChartsImpl(data);
    }, 0);
}

function renderAnalyticsChartsImpl(data) {
    // Traffic: allowed + denied stacked as an area chart. The stack top
    // (allowed + denied) is the total request volume per bucket.
    const trafficCtx = document.getElementById('traffic-chart');
    if (trafficCtx) {
        if (trafficChart) trafficChart.destroy();
        trafficChart = new Chart(trafficCtx, {
            type: 'line',
            data: {
                labels: data.trafficLabels,
                datasets: [
                    {
                        label: 'Allowed',
                        data: data.trafficAllowed,
                        borderColor: '#22C55E',
                        backgroundColor: 'rgba(34, 197, 94, 0.25)',
                        fill: true,
                        tension: 0.3,
                        pointRadius: 0,
                    },
                    {
                        label: 'Denied',
                        data: data.trafficDenied,
                        borderColor: '#DC2626',
                        backgroundColor: 'rgba(220, 38, 38, 0.25)',
                        fill: true,
                        tension: 0.3,
                        pointRadius: 0,
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: { position: 'bottom', labels: { boxWidth: 12, padding: 10 } }
                },
                scales: {
                    y: { stacked: true, beginAtZero: true }
                }
            }
        });
    }
}
