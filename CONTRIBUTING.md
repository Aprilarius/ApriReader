# Contributing

Thank you for helping improve ApriReader.

Before changing product behavior, read `AGENTS.md`, the product and UI
specifications, architecture, dependency policy, and roadmap.

- Treat every book and language package as untrusted input.
- Never execute embedded scripts or allow implicit external requests.
- Keep source books in place and user data local by default.
- Do not add a dependency without reviewing its license.
- Do not add Steamworks files, telemetry, catalogs, TTS, models, or generated
  covers to the public repository.
- Add focused tests and update documentation with each behavioral change.

Run `pnpm check` before submitting a change. It covers formatting, lint,
TypeScript, UI tests, production build, Rust formatting, Clippy, Rust tests,
security boundaries, SBOM, and the third-party license bundle.
