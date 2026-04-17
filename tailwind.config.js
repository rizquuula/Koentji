/** @type {import('tailwindcss').Config} */
//
// Semantic design tokens.
//
// The previous config was an empty `theme: { extend: {} }` — every
// `blue-600` / `gray-500` / `rounded-lg` was inlined in ~40 files.
// That makes the admin dashboard dependent on Tailwind's palette
// names rather than on words from its own product (what's a "brand"
// blue? what radius means "card"? what's an "accent"?). The next
// primitive extraction (6.2) needs a shared vocabulary to compose
// from.
//
// Tokens are additive: the raw palette stays available for migration,
// so this commit does not itself touch any component — it just adds
// the semantic layer that 6.2 + onward can spend.
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        // Brand accents. The dashboard's visual identity lives on the
        // blue primary action (Save, Log in, primary CTAs). Maps to
        // Tailwind's existing `blue` so the migration stays mechanical.
        brand: {
          50:  "#eff6ff",   // blue-50
          100: "#dbeafe",   // blue-100
          500: "#3b82f6",   // blue-500  — focus ring
          600: "#2563eb",   // blue-600  — primary button background
          700: "#1d4ed8",   // blue-700  — primary button hover
          800: "#1e40af",   // blue-800  — link hover / emphasis
        },
        // Neutral surface + text scale. Almost everything in the UI
        // picks from this ladder.
        surface: {
          base:    "#ffffff", // the page background (cards, dialogs)
          subtle:  "#f9fafb", // gray-50   — page wrapper, muted panels
          muted:   "#f3f4f6", // gray-100  — hover/zebra rows, chips
          border:  "#e5e7eb", // gray-200  — divider borders
          strong:  "#d1d5db", // gray-300  — input borders
        },
        ink: {
          disabled: "#9ca3af", // gray-400
          muted:    "#6b7280", // gray-500 — secondary copy, helper text
          subdued:  "#4b5563", // gray-600
          body:     "#374151", // gray-700 — body copy default
          heading:  "#111827", // gray-900 — page titles, table headers
          inverse:  "#ffffff", // text on brand background
        },
        // Feedback colors — surface up in form validation, toasts,
        // badges. Mapped to Tailwind's red/yellow/green at saturations
        // matching the existing inline usage.
        feedback: {
          danger:        "#dc2626", // red-600 — destructive buttons, errors
          "danger-ink":  "#991b1b", // red-800 — error text on light bg
          warning:       "#d97706", // amber-600 — warnings
          "warning-ink": "#92400e", // amber-800
          success:       "#16a34a", // green-600 — success toasts
          "success-ink": "#166534", // green-800
        },
      },
      borderRadius: {
        // Map to existing usage: `rounded` (small controls), `rounded-lg`
        // (cards, modals).
        control: "0.375rem", // rounded-md — buttons, inputs
        card:    "0.5rem",   // rounded-lg — surfaces, modals
      },
      boxShadow: {
        // Named shadows — the only two depths the dashboard actually
        // uses. Fewer shadow options = more consistent visual weight.
        raised:  "0 1px 2px 0 rgb(0 0 0 / 0.05)",          // shadow-sm
        overlay: "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)", // shadow-xl — modals
      },
      transitionDuration: {
        quick:   "150ms", // hover / focus feedback
        settled: "300ms", // slide-in, fade-in, modal enter
      },
      keyframes: {
        // Toast component references `animate-slide-in` but the CSS
        // was never defined (F3). Declare it here so the utility
        // resolves and the toast animation actually plays. The
        // component-level fix lands in 6.3 — this keeps the keyframe
        // living with the rest of the design tokens.
        "slide-in": {
          "0%":   { transform: "translateX(100%)", opacity: "0" },
          "100%": { transform: "translateX(0)",    opacity: "1" },
        },
      },
      animation: {
        "slide-in": "slide-in 300ms cubic-bezier(0.16, 1, 0.3, 1)",
      },
    },
  },
  plugins: [],
}
