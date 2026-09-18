# Publishing the CLI

> Status: **0.3.0 prepared, not yet published.** The version is bumped in
> `Cargo.toml`; the CI publish workflow (`publish.yml`) releases it when
> `main` is green and the version is not already on crates.io.
>
> Last updated: 2026-09-18.

## Current state

- `actos-cli` **0.2.1** is on crates.io (2026-09-07).
- The `actos` Rust SDK **0.3.0** is on crates.io (2026-09-18), together with
  `actos-types` **0.3.0**.
- The CLI depends on the registry version:
  `actos-sdk = { package = "actos", version = "0.3", default-features = false }`.
- `CARGO_REGISTRY_TOKEN` is configured as a GitHub secret on
  `actos-dev/cli`.
- The crate name is `actos-cli`; `[[bin]] name = "actos"` keeps the command
  as `actos`.

## Resolved blockers (kept for history)

### Path dependency on `actos-types` — resolved

The CLI used to depend on `actos-types` through a local `path`. `cargo
publish` rejects path dependencies, so `cargo install actos` did not work.
`actos-types` (and the `actos` SDK that re-exports it) is now published to
crates.io and the CLI depends on it by version. `cargo publish --dry-run`
passes.

### crates.io name collision — resolved

Both the SDK and the CLI once declared `name = "actos"`. The SDK keeps
`actos`; the CLI publishes as `actos-cli` and installs the `actos` binary:

```toml
[package]
name = "actos-cli"

[[bin]]
name = "actos"
path = "src/main.rs"
```

### Turkish `description` — resolved

The package `description` is English; the crate page is outward-facing and
the project language is English.

## Publishing checklist

1. Confirm the backend the release targets is live.
2. Confirm `actos`/`actos-types` of the matching version are on crates.io.
3. `cargo fmt --check`
4. `cargo clippy --all-targets --all-features -- -D warnings`
5. `cargo test --all-features`
6. `cargo build --all-features` and `cargo build --no-default-features`
7. `cargo publish --dry-run`
8. Let the CI publish workflow release the tag.

## Publish discipline — lessons from the 0.2.0 incident (2026-09-07)

### 1. The version is bumped in the same round as the feature

0.2.0 was cut before the `actos update` command existed: the version bump
was pushed, automation ran, and crates.io received code **without** the
`update` command. Users running `actos update` got "unrecognized
subcommand"; 0.2.1 had to be released to fix it.

Rule: a user-visible feature and its version bump land in the same round.
The automation publishes the moment it sees a new version, so accumulating
unversioned features means cutting a release with missing code.

### 2. Tests never embed a version number

`tests/update_test.rs` hard-coded `"0.2.0"`; the 0.2.1 bump broke it (the
binary reported 0.2.1 while the test expected 0.2.0).

Rule: version-dependent tests use `env!("CARGO_PKG_VERSION")`. A literal
version string in a test is forbidden — it breaks on every bump.

## History — the CLI now uses the Rust SDK

The CLI once carried its own transport layer (retry, rate limiting,
idempotency keys, cursor pagination) beside the Rust SDK, duplicating the
same policy in two places. The CLI now depends on the `actos` crate and
`src/client/` is gone; what remains is argument parsing, output formatting
and the TUI.
