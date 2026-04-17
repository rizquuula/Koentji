use leptos::prelude::*;

/// Table scaffold: header row from a `Vec<&str>`, body from the caller's
/// `children`. Collapses the shared `<div class="bg-white shadow rounded">
/// <table><thead>…</thead><tbody>…</tbody></table></div>` boilerplate
/// that each CRUD page was copying verbatim. `scope="col"` on every header
/// is a freebie accessibility win — a screen reader can now associate a
/// cell with its column.
///
/// Callers compose `<tr><td>…</td></tr>` inside; the primitive does not
/// know about item types. Building a fully generic `DataTable<Item>` would
/// buy the reader nothing over a free `Vec<_>::into_iter().map(…)` at the
/// call site and would fight Leptos's generic-component ergonomics.
#[component]
pub fn DataTable(headers: Vec<&'static str>, children: Children) -> impl IntoView {
    view! {
        <div class="bg-surface-base rounded-card shadow-raised border border-surface-border overflow-hidden">
            <table class="w-full">
                <thead class="bg-surface-subtle">
                    <tr>
                        {headers.into_iter().map(|h| view! {
                            <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-ink-muted uppercase">
                                {h}
                            </th>
                        }).collect::<Vec<_>>()}
                    </tr>
                </thead>
                <tbody class="divide-y divide-surface-border">
                    {children()}
                </tbody>
            </table>
        </div>
    }
}
