# Yamalyze

A client-side semantic YAML diff tool. Rust compiled to WebAssembly powers the diff engine, vanilla JavaScript handles the UI. No backend server, database, or external API — everything runs in the browser.

## Features

- Semantic YAML comparison — understands YAML structure (mappings, sequences, scalars) rather than comparing text lines
- Diff types: `Unchanged`, `Additions`, `Deletions`, `Modified`
- LCS-based array diffing — insertions and removals mid-sequence are detected correctly using Longest Common Subsequence matching
- Recursive diff tree — nested objects and arrays produce a hierarchical diff with `has_diff` propagated from children
- Auto-diff with debounce — comparison runs automatically as you type (400ms debounce)
- File upload — load `.yaml`/`.yml` files from disk into either editor panel
- Line numbers with error highlighting — gutter synced to textarea scroll, error lines highlighted red
- Inline scalar display — scalar key-value pairs shown inline next to their key, only nested objects/arrays are collapsible
- Collapsible diff output — unchanged keys collapsed by default, additions (green), deletions (red), modified (amber) expanded
- Clickable diff filters — click Additions, Deletions, or Modified in the summary bar to filter the tree to only that type; click again to show all
- Simultaneous error reporting — both YAML parse errors shown at once if both inputs are invalid
- Runs entirely in the browser via WebAssembly

## Prerequisites

- Rust toolchain with `wasm-pack`
- Node.js 18+

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

- `lib.rs` — WASM entry point. Exports `diff(yone, ytwo)` which parses and diffs two YAML strings.
- `diff.rs` — Recursive diff engine. Compares mappings key-by-key, sequences via LCS, and scalars by value equality.

### JavaScript Frontend (`pages/`)

- `index.js` — Debounced auto-diff, file upload, line number gutter with error highlighting, recursive diff tree rendering with collapsible nodes, clickable diff type filters, localStorage persistence.
- `style.css` — Tailwind CSS v4 with custom classes for editor gutter, diff tree color coding, and collapsible details/summary.

### Build Pipeline

Webpack with `@wasm-tool/wasm-pack-plugin` compiles Rust to WASM during the build. Output goes to `_site/`.

## Deployment

GitHub Actions runs lint and build on push to `main` and on PRs. Deploy to GitHub Pages happens on push to `main` after both jobs pass.

## License

See [LICENSE](LICENSE) for details.
