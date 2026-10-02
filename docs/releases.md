# Releases

Only `vMAJOR.MINOR.PATCH` tags publish releases. Pushes to `main` run validation and produce a development installer artifact without creating a release.

## Signing setup

Create an updater key with `pnpm tauri signer generate -w <path-outside-the-repository>`. Keep a secure backup. Never put private keys, passwords, or signing certificates in Git.

Configure the GitHub repository with:

| Setting                              | Type             | Purpose                                    |
| ------------------------------------ | ---------------- | ------------------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`          | Actions secret   | Contents of the private updater key        |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Actions secret   | Key password, if one was set               |
| `TAURI_SIGNING_PUBLIC_KEY`           | Actions variable | Base64 public key from the public key file |

The release workflow requires a valid public key. It constructs an ignored Tauri configuration containing the public key and the GitHub `latest.json` endpoint. The private key is passed only to the installer-signing step. Development builds omit updater configuration and cannot install unverified updates.

Updater signing does not provide Windows Authenticode publisher identity. Configure a Windows signing certificate separately if trusted-publisher installation is required.

## Release procedure

1. Ensure `main` is clean, pushed, and synchronized with `origin/main`, and that its GitHub build passed.
2. Review changes and verify Google sign-in, playlist synchronization, settings, curation, and Discord against configured accounts. Confirm that no credentials or personal data are staged.
3. Run `pnpm release patch`, `minor`, or `major` on Windows. The command checks formatting, linting, types, tests, and the frontend build before and after the version bump. It builds an NSIS installer and smoke-tests the release executable before committing and tagging.
4. Watch the Release workflow. It validates the tag against all manifests, runs native smoke tests, builds the signed installer, and uploads the installer, signature, and updater metadata to a draft release. Installation, reinstallation, startup, and uninstallation are checked on the disposable runner before publishing the release.
5. Verify the published installer and add concise, user-facing release notes describing the changes.

The version command permits only version-field additions in its staged diff. If validation fails, it leaves reviewable working changes and creates no tag. If an atomic push fails, the local commit and tag remain; resolve the push failure without rerunning the version bump or rewriting published tags.

Check that the installed application starts, loads settings, initializes its database and tray, and connects to production resources. Close other Resonance instances before smoke tests; the application enforces a single instance.

## Updating

Configured releases check for updates at startup and expose **Check for updates** in Settings. Installation requires an explicit user action and verifies the update signature before restarting.

Keep the same signing key across releases. Key rotation needs a deliberate migration through a release trusted by existing installations. Do not replace a public key casually or disable verification to work around a signing failure.
