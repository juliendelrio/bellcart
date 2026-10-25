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

On Windows, Rust needs the Visual Studio Build Tools (the C++ workload
and a Windows SDK). Run Cargo from PowerShell or a Developer prompt: in
Git Bash, the `link` of coreutils comes before the Microsoft linker in
the `PATH`, and linking fails with `link: extra operand`.

Start with the [specification](docs/SPEC-V1.md) and the
[decision log](docs/DECISIONS.md). For anything larger than a fix, open an
issue first so we agree on the approach.

## Branches

Work happens on a branch, merged into `main` through a pull request.
Name it `<type>/<issue>-<short-description>`, in lowercase ASCII with
hyphens:

| Type | For |
| --- | --- |
| `feat` | a new capability of the engine or the interface |
| `fix` | a bug fix |
| `pack` | rules, recipes and questions under `packs/` |
| `i18n` | a new language or translation fixes |
| `docs` | documentation only |
| `ci` | workflows and release tooling |
| `chore` | dependencies, tooling, clean-ups |

Examples: `feat/1-review-signatures`, `pack/4-seed-rules`,
`fix/typo-in-readme`. The issue number is optional for small fixes. For
anything else, open the issue first, put its number in the branch name,
and write `Closes #<issue>` in the pull request. CI checks the name of
every pull request branch.

A few rules are not negotiable: Bellcart never modifies the scanned tree,
never reads file contents, never makes a network request the user did not
configure, and packs can never carry commands.

## Licence

Code and packs are under different licences (see [LICENSE.md](LICENSE.md)):

- A contribution to a pack, under `packs/`, is licensed under that pack's
  licence: [Apache-2.0](packs/LICENSE) for the core pack.
- A contribution to anything else is licensed under the
  [Bellcart Sustainable Use License](LICENSE.md), or the
  [Bellcart Enterprise License](LICENSE_EE.md) for enterprise components.
  You keep the copyright of your contribution, and you also grant the
  maintainer a perpetual, worldwide, royalty-free licence to use,
  modify and relicense it, including under the enterprise and other
  commercial licences. This lets the project change or add licences
  without asking every contributor again.

If you cannot agree to the second point, contribute to packs: that is
where most of Bellcart's knowledge lives.
