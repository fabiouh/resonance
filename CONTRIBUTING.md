# Contributing

## Setup

Use Windows x64 with Node.js 24, pnpm 10, stable Rust, Visual Studio's C++ desktop workload, a Windows SDK, and WebView2. Clone the repository, branch from `main`, then run `pnpm install --frozen-lockfile` and `pnpm tauri dev`.

## Development and testing

- `pnpm format` and `cargo fmt --manifest-path src-tauri/Cargo.toml` format the source.
- `pnpm validate` runs frontend formatting, Svelte/TypeScript checks, linting, unit tests, the frontend build, Rust formatting, Clippy, and Rust tests.
- `pnpm tauri build --debug --no-bundle` followed by `pnpm test:desktop` checks the native window, playlist creation/renaming/deletion, history import, and settings. Close other Resonance instances first. Test profiles live in the temporary directory.
- `pnpm test:desktop --online` additionally checks public YouTube metadata, playback, and listening history. It needs network access and is excluded from CI to avoid provider availability affecting offline checks.
- `pnpm tauri build` produces the Windows installer. `pnpm test:desktop:release` checks its release executable without modifying library content.

Tests use isolated fixtures. Never add sample playlists or placeholder statistics to production paths. The native debug test profile override is unavailable in release builds.

CI also installs, starts, reinstalls, and uninstalls the Windows package on a disposable runner. Reinstallation of the same version does not replace testing an upgrade between signed releases.

Before release, use a dedicated test playlist to check Google sign-in, refresh after restarting, synchronization, remote renaming/deletion, and curation previews. Confirm that shared source tracks and manually added target tracks survive curation. Check Discord activity during playback and after pause, disconnect, and Discord restart. Test signed updates between two published versions using the same signing key. Enter credentials locally in Settings or `.env`; never include them in test reports.

Generate application icons from `assets/icon.svg` with `pnpm tauri icon assets/icon.svg`. Commit only the configured Windows icon files.

## Pull requests

Keep changes focused. Explain the problem, resulting behavior, and validation; include screenshots for interface changes. Add regression tests for changes to synchronization, ownership, authentication, and persistence.

Use scoped conventional commits, such as `fix(sync): preserve shared tracks` or `chore(deps): update dependencies`. Inspect staged changes for credentials, personal data, build output, and unrelated modifications before committing.

Discuss substantial product changes in an issue before starting work. Report vulnerabilities using [the security policy](SECURITY.md). Release maintainers should follow [the release guide](docs/releases.md).
