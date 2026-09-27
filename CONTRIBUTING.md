# Contributing

Issues and pull requests are welcome. Include a small reproduction for bugs;
check existing issues before proposing a feature. Keep changes focused and simple.

Editio uses the private `tapp-ui` library, kept as a sibling checkout. A public
checkout of Editio alone cannot build; framework access is not provided by this
repository. There is no CI. Builds and publishing happen locally.

## Local development (with framework access)

```sh
cargo run -- notes.md
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

Normal builds target only your machine. Use
`cargo run --locked --manifest-path tools/release/Cargo.toml -- refresh-local`
when intentionally refreshing the local optimized executable. All application
code, tests and development tools are Rust; the shell installer and Homebrew
formula are small platform packaging adapters.

Record user-visible changes under `Unreleased` in [CHANGELOG.md](CHANGELOG.md),
using Added, Changed or Fixed as appropriate. Do not add CI or automatic release
build hooks. Never commit `.local`, credentials, drafts or generated binaries.

- [Local release workflow](docs/distribution.md#local-publishing)
- [Validation notes](docs/validation.md)
- [Embedding API](docs/embedding.md)
- [Product spec](docs/spec.md)
