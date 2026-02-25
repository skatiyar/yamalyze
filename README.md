# Yamalyze

A client-side semantic YAML diff tool. Rust compiled to WebAssembly powers the diff engine, vanilla JavaScript handles the UI. No backend server, database, or external API — everything runs in the browser.

## Features

- Semantic YAML comparison — understands YAML structure (mappings, sequences, scalars) rather than comparing text lines
- Diff types: `Unchanged`, `Additions`, `Deletions`, `Modified`
- Myers-based array diffing — insertions and removals mid-sequence are detected using the Myers O(ND) diff algorithm (`similar` crate), replacing the old O(n\*m) LCS approach. Falls back to positional comparison for extremely large sequences.
- Recursive diff tree — nested objects and arrays produce a hierarchical diff with `has_diff` propagated from children. Recursion capped at 128 levels to prevent stack overflow.
- Large file support (up to 100MB) — tiered behavior: auto-diff for files under 10MB, explicit "Run Diff" button for larger files, read-only mode for 50MB+, rejection above 100MB
- Auto-diff with debounce — comparison runs automatically as you type (400ms debounce) for small/medium files
- File upload — load `.yaml`/`.yml` files from disk into either editor panel with size validation
- Line numbers with error highlighting — gutter synced to textarea scroll, error lines highlighted red
- Inline scalar display — scalar key-value pairs shown inline next to their key, only nested objects/arrays are collapsible
- Expandable additions/deletions — added or removed keys with nested structure are shown as collapsible trees, not flat `{}` / `[...]`
- Collapsible diff output — unchanged keys collapsed by default, additions (green), deletions (red), modified (amber) expanded
- Clickable diff filters — click Additions, Deletions, or Modified in the summary bar to filter the tree to only that type; click again to show all
- Chunked diffing — mappings are diffed per top-level key with UI yields between chunks, keeping the browser responsive
- Progress loader — spinner with stage text ("Parsing YAML...", "Computing diff (i/N)...", "Rendering...") shown during processing
- Storage quota warning — amber overlay on the affected editor when localStorage is full, instead of silent failure
- Simultaneous error reporting — both YAML parse errors shown at once if both inputs are invalid
- Runs entirely in the browser via WebAssembly

## Prerequisites

- Rust toolchain with `wasm-pack`
- Node.js 24+

## Getting Started

```bash
npm install          # Install JS dependencies
npm run serve        # Start webpack dev server (compiles Rust to WASM automatically)
```

Open http://localhost:8080, paste or upload two YAML documents — the diff tree renders automatically as you type.

## Build

```bash
npm run build        # Production build → _site/ directory
```

## Linting

```bash
npm run lint          # Run all linters (ESLint + Prettier + cargo fmt + clippy)
npm run lint:fix      # Auto-fix all (ESLint + Prettier + cargo fmt)
```

## Architecture

### Rust/WASM Core (`src/`)

- `lib.rs` — WASM entry point. Exports a chunked diff API (`diff_init`, `diff_key`, `diff_stored`, `diff_cleanup`) for large files.
- `diff.rs` — Recursive diff engine. Compares mappings key-by-key, sequences via Myers diff algorithm (`similar` crate), and scalars by value equality. Additions/deletions of complex values produce full recursive child trees for expandable rendering. Diff results are serialized to plain JS objects in Rust to minimize WASM boundary overhead.

### JavaScript Frontend (`pages/`)

- `index.js` — Async chunked diff with progress loader, debounced auto-diff (small files) or explicit "Run Diff" button (large files), file upload with size tiers, line number gutter with error highlighting, recursive diff tree rendering with collapsible nodes, clickable diff type filters, localStorage persistence (small files only).
- `style.css` — Tailwind CSS v4 with custom classes for editor gutter, diff tree color coding, and collapsible details/summary.

### Build Pipeline

Webpack with `@wasm-tool/wasm-pack-plugin` compiles Rust to WASM during the build. Output goes to `_site/`.

## Deployment

GitHub Actions runs lint and build on push to `main` and on PRs. Deploy to GitHub Pages happens on push to `main` after both jobs pass.

## License

See [LICENSE](LICENSE) for details.
