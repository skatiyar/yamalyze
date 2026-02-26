# Contributing to Yamalyze

Thanks for your interest in contributing! This guide will help you get set up and familiar with the project conventions.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) toolchain (stable) with `wasm-pack`
- [Node.js](https://nodejs.org/) 24+

### Setup

```bash
npm install          # Install JS dependencies + set up husky pre-commit hooks
npm run serve        # Start webpack dev server (compiles Rust to WASM automatically)
```

Open http://localhost:8080 to see the app running locally.

## Development Workflow

1. Create a branch off `main`:
   - `feature/short-description` for new features
   - `fix/short-description` for bug fixes
2. Make your changes
3. Run `npm run lint` (or let the pre-commit hook catch issues)
4. Push your branch and open a pull request against `main`

### Useful Commands

```bash
npm run serve        # Dev server with hot reload
npm run build        # Production build → _site/
npm run lint         # Run all linters (ESLint + Prettier + cargo fmt + clippy)
npm run lint:fix     # Auto-fix all lint issues
```

## Code Style

Linting is enforced by pre-commit hooks (husky + lint-staged), so issues are caught before they reach CI.

- **JavaScript**: ESLint + Prettier. Run `npm run lint:js` to check, `npm run lint:fix` to auto-fix.
- **Rust**: cargo fmt + clippy (targeting `wasm32-unknown-unknown`). Run `npm run lint:rs` to check.
- **CSS**: Tailwind CSS v4. Use `@apply` in `pages/style.css` for reusable component styles rather than inline classes.
- **Markdown/JSON/YAML**: Prettier auto-formats these via lint-staged.

## Commit Messages

Use the format `type: subject` with lowercase, no trailing period:

- `feat: add copy-to-clipboard button`
- `fix: correct gutter alignment on scroll`
- `docs: update README with new tier limits`
- `refactor: extract diff rendering into helper`
- `chore: bump wasm-bindgen to 0.2.112`

## Architecture Guidelines

- **Diff engine logic** goes in Rust (`src/`). All diffing, parsing, and serialization happens in WASM.
- **UI and rendering** stays in vanilla JavaScript (`pages/index.js`). No frameworks (React, Vue, etc.).
- **Styling** uses Tailwind CSS with custom classes in `pages/style.css`.
- **No external JS runtime dependencies.** The goal is a minimal, self-contained tool.

## Submitting a Pull Request

1. Ensure `npm run lint` passes
2. Ensure `npm run build` succeeds
3. Describe your changes clearly in the PR description
4. Reference any related issues (`Fixes #123`)
5. Include screenshots for UI changes
6. Test in both Chrome and Firefox (WebAssembly behavior can vary)

## Reporting Bugs

Please use the [bug report template](https://github.com/skatiyar/yamalyze/issues/new?template=bug_report.md) when filing issues. Include steps to reproduce, expected vs actual behavior, and your browser/OS.
