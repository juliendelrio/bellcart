# Contributing to Bellcart

Thank you for helping people move house without the junk.

The most useful contribution is knowledge: where an application keeps its
data, and what it takes to move it. That is a YAML file, not Rust.

## Add or fix a recipe or a rule

1. Read the [pack guide](docs/PACKS.md).
2. Edit or add a file under `packs/core/`.
3. Run `cargo test`. It validates every pack file, checks that texts are
   translated, and runs the scan specification.
4. In the pull request, say which version of the application and of
   Windows you checked it on.

For a rule that decides `leave`, add a case to `tests/scan.rs` showing a
folder that matches and a look-alike that must not. A wrong `leave` loses
data, so these rules are reviewed strictly.

## Add a language

1. Copy `locales/en/bellcart.ftl` to `locales/<code>/bellcart.ftl` and
   translate the values. Keep the keys.
2. Add the language to `LOCALES` in `src/bin/bellcart/i18n.rs`.
3. Add it to the language lists in `tests/core_pack.rs`.

## Change the code

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass. The engine must keep building on Linux and macOS even
though the product targets Windows.

Start with the [specification](docs/SPEC-V1.md) and the
[decision log](docs/DECISIONS.md). For anything larger than a fix, open an
issue first so we agree on the approach.

A few rules are not negotiable: Bellcart never modifies the scanned tree,
never reads file contents, never makes a network request the user did not
configure, and packs can never carry commands.

## Licence

By contributing you agree that your contribution is licensed under
[Apache-2.0](LICENSE), like the rest of the project.
