# Contributing

Resonance is in early development. Application source and build commands are not yet available.

## Setup

Clone the repository and create a branch from `main`. Keep changes focused and avoid unrelated formatting or dependency updates.

## Validation

Run the format, lint, typecheck, test, and build commands provided by the components you change. Include relevant regression tests for behavior changes. Until build tooling is added, check patches with `git diff --check` and review documentation links and workflow syntax.

Never include credentials, account data, local configuration, or build outputs. Inspect staged changes before committing.

## Pull requests

Explain the problem, the resulting behavior, and how the change was verified. Include screenshots for visible interface changes. Use concise conventional commit messages, such as `fix: preserve playlist ordering`.

Discuss substantial product changes in an issue before starting work. Report vulnerabilities using [the security policy](SECURITY.md).
