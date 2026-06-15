// Analytics chart renderer. The Rust side calls the exported
// `renderAnalyticsCharts(dataJson)` global via a typed `#[wasm_bindgen]`
// extern — no `eval`, no `window.__chartData` smuggling.
//
// The argument is a JSON *string* so the wasm → JS crossing stays a
// single primitive value. The body defers via setTimeout so the canvas
// is live in the DOM before Chart.js probes it, and destroys the prior
// instance before re-creating so range switches don't leak canvases.

let trafficChart = null;
let latencyChart = null;
let denialsChart = null;
let usageChart = null;

function renderUsageChart(dataJson) {
    setTimeout(function() {
        if (typeof dataJson !== 'string' || dataJson.length === 0) return;
        let data;
        try {
            data = JSON.parse(dataJson);
        } catch (_) {
            return;
        }

        const ctx = document.getElementById('usage-chart');
        if (!ctx) return;

        const labels = formatTrafficLabels(data.ts, data.rangeIs24h, data.bucketSecs);
        if (usageChart) usageChart.destroy();
        usageChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: labels,
                datasets: [
                    {
                        label: 'Usage consumed',
                        data: data.usage,
                        borderColor: '#8B5CF6',
                        backgroundColor: 'rgba(139, 92, 246, 0.2)',
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
                    y: { beginAtZero: true }
                }
            }
        });
    }, 0);
}

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

// Format the raw unix-ms bucket timestamps into axis labels in the *browser's*
// local timezone. `new Date(ms)` and the `toLocale*` formatters both resolve to
// the viewer's zone, so the axis follows whoever is looking at it. The label
// shape follows the *bucket* size, not just the window:
//   - daily or coarser buckets (>= 1 day): a bare date "DD Mon" — the time of
//     day is meaningless once a bucket spans a whole day or more.
//   - sub-daily buckets on the 24h window: a bare "HH:MM".
//   - sub-daily buckets on a wider window: "DD Mon HH:MM" so a time isn't
//     ambiguous across midnight.
// `bucketSecs` is the *effective* bucket size after server-side clamping, so the
// labels always match what was actually rendered. The traffic and latency
// charts share one label array (same bucket grid), so this runs once per render.
function formatTrafficLabels(tsList, rangeIs24h, bucketSecs) {
    if (!Array.isArray(tsList)) return [];
    const timeOpts = { hour: '2-digit', minute: '2-digit', hour12: false };
    const dayOpts = { day: '2-digit', month: 'short' };
    const dailyOrCoarser = typeof bucketSecs === 'number' && bucketSecs >= 86400;
    return tsList.map(function(ms) {
        const d = new Date(ms);
        if (dailyOrCoarser) return d.toLocaleDateString([], dayOpts);
        const time = d.toLocaleTimeString([], timeOpts);
        if (rangeIs24h) return time;
        return d.toLocaleDateString([], dayOpts) + ' ' + time;
    });
}

function renderAnalyticsChartsImpl(data) {
    const trafficLabels = formatTrafficLabels(data.trafficTs, data.rangeIs24h, data.bucketSecs);

    // Traffic: allowed + denied stacked as an area chart. The stack top
    // (allowed + denied) is the total request volume per bucket.
    const trafficCtx = document.getElementById('traffic-chart');
    if (trafficCtx) {
        if (trafficChart) trafficChart.destroy();
        trafficChart = new Chart(trafficCtx, {
            type: 'line',
            data: {
                labels: trafficLabels,
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

    // Latency: p50/p95/p99 as three lines sharing the traffic x-axis. Empty
    // buckets arrive as `null`; `spanGaps: false` breaks the line there
    // rather than interpolating a phantom latency across idle windows.
    const latencyCtx = document.getElementById('latency-chart');
    if (latencyCtx) {
        if (latencyChart) latencyChart.destroy();
        latencyChart = new Chart(latencyCtx, {
            type: 'line',
            data: {
                labels: trafficLabels,
                datasets: [
                    {
                        label: 'p50',
                        data: data.latencyP50,
                        borderColor: '#3B82F6',
                        backgroundColor: '#3B82F6',
                        fill: false,
                        tension: 0.3,
                        pointRadius: 0,
                        spanGaps: false,
                    },
                    {
                        label: 'p95',
                        data: data.latencyP95,
                        borderColor: '#F59E0B',
                        backgroundColor: '#F59E0B',
                        fill: false,
                        tension: 0.3,
                        pointRadius: 0,
                        spanGaps: false,
                    },
                    {
                        label: 'p99',
                        data: data.latencyP99,
                        borderColor: '#DC2626',
                        backgroundColor: '#DC2626',
                        fill: false,
                        tension: 0.3,
                        pointRadius: 0,
                        spanGaps: false,
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
                    y: { beginAtZero: true, title: { display: true, text: 'ms' } }
                }
            }
        });
    }

    // Denials by reason: a doughnut. The canvas only exists when there are
    // denials (the Leptos side renders an empty-state text instead), so the
    // `getElementById` guard doubles as the empty-window short-circuit.
    const denialsCtx = document.getElementById('denials-chart');
    if (denialsCtx) {
        if (denialsChart) denialsChart.destroy();
        denialsChart = new Chart(denialsCtx, {
            type: 'doughnut',
            data: {
                labels: data.denialLabels,
                datasets: [
                    {
                        data: data.denialCounts,
                        backgroundColor: data.denialColors,
                        borderWidth: 1,
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        position: 'bottom',
                        labels: {
                            boxWidth: 12,
                            padding: 10,
                            // Append each slice's count to its legend label so
                            // the breakdown reads without hovering.
                            generateLabels: function(chart) {
                                const d = chart.data;
                                return d.labels.map(function(label, i) {
                                    const value = d.datasets[0].data[i];
                                    const color = d.datasets[0].backgroundColor[i];
                                    return {
                                        text: label + ' (' + value + ')',
                                        fillStyle: color,
                                        strokeStyle: color,
                                        index: i,
                                    };
                                });
                            }
                        }
                    }
                }
            }
        });
    }
}
