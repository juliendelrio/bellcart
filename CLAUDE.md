# Bellcart

A migration assistant for people changing PC: it scans the old machine and
produces a plan of what to migrate, archive or leave. Windows 10 and 11,
one portable executable, Rust.

Read `docs/SPEC-V1.md` before any non-trivial change, and
`docs/DECISIONS.md` before questioning a design choice. `docs/ROADMAP.md`
says what comes next.

## Working with the maintainer

- The maintainer is French-speaking and new to Rust. Talk to him in French.
  Explain Rust-specific choices briefly when they are not obvious.
- Code, comments, commit messages, docs and pack content are in English.
  `README.fr.md` mirrors `README.md`: update both together.
- Never use the em dash character in any prose you write, in either
  language. Use a comma, a colon, parentheses or two sentences.

## Rules that must never be broken

1. **Read-only.** Bellcart never writes, moves or deletes anything in the
   tree it scans. V1 produces a plan and a notebook, nothing else.
2. **A wrong `leave` is the worst bug.** It costs someone their data. When
   a rule or a piece of logic could send a personal folder to `leave`, it
   must go to `review` instead. See "Asymmetric prudence" in the spec.
3. **Never read file contents.** Directory listings only. Do not open
   files: on Windows that can also trigger a cloud download.
4. **No network by default.** Only an arbiter the user configured may make
   requests, and only to the endpoint the user gave.
5. **Packs are data.** No step type may run an arbitrary command. A new
   executable primitive is a named, reviewed enum variant in `src/pack.rs`.
6. **No `unsafe` in this package** (`forbid` in `Cargo.toml`). Windows API
   calls go in a separate member crate, added in milestone M5.
7. **No `unwrap`, `expect` or `panic!` outside tests.** Clippy denies them.
   Return an error.

## Commands

Run all three before saying a change is done:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

After changing a type in `src/pack.rs` or `src/plan.rs`, regenerate the
schemas, then update `docs/PACKS.md`:

```sh
BELLCART_UPDATE_SCHEMAS=1 cargo test --test schemas
```

Try the binary: `cargo run -- scan <folder>`, `cargo run -- --lang fr packs`.

## Layout

```text
src/lib.rs            engine (library): everything that decides
src/tree.rs           read-only view of a directory tree; MemTree for tests
src/scan.rs           tree + rules -> plan
src/plan.rs           the plan format (published, versioned)
src/pack.rs           pack format, loader, rule matching
src/signature.rs      text summary of a review unit (the arbiter's state)
src/recipe.rs         recipe variables, detection, steps settled in the plan
src/notebook.rs       the tasks of the migration notebook
src/arbiter.rs        model contract (System One shaped), `none` backend
src/schema.rs         JSON Schema generation
src/bin/bellcart/     CLI: argument parsing, i18n, text rendering only
packs/core/           rules, recipes, questions embedded in the binary
locales/<lang>/       interface strings (Fluent)
schemas/              generated, checked in, do not edit by hand
tests/scan.rs         the behaviour specification
tests/signatures.rs   what a signature says, and its budget
tests/notebook.rs     what the migration notebook asks the user to do
tests/core_pack.rs    checks on the shipped pack and its translations
tests/cli.rs          end-to-end on the real binary
```

The engine never prints and never knows about languages: it returns data
and translation keys. Rendering and wording belong to the front ends.

## How to make a change

- **Branch**: work on `<type>/<issue>-<short-description>` (types:
  `feat`, `fix`, `pack`, `i18n`, `docs`, `ci`, `chore`), never directly on
  `main` unless the maintainer asks. Details in `CONTRIBUTING.md`.

- **Behaviour change in the scan or rules**: write or change the case in
  `tests/scan.rs` first, using `MemTree`. Those tests are the spec, and the
  maintainer reads them to check what the tool does. Name them as
  sentences.
- **New rule or recipe**: edit YAML under `packs/core/`, add a scan test
  for any new `leave` rule that shows both a match and a look-alike folder
  that must not match. Follow `docs/PACKS.md`.
- **New user-facing string**: add the key to `locales/en/bellcart.ftl` and
  to every other language. A test fails if French misses a key.
- **Design decision**: add an entry at the end of `docs/DECISIONS.md`.
- Keep dependencies few. Ask before adding one that is not small and
  widely used.

## Windows facts to keep in mind

- The engine must build and pass tests on Linux and macOS. Nothing outside
  the future Windows crate may assume Windows.
- Plan paths are relative to the scan root and use `/`.
- Folder names are localised on screen but not on disk. Resolve special
  folders by identifier, never by display name.
- Links and junctions are reported as `Kind::Other` and never followed.
- Matching is case-insensitive, as the file system is.
- CI is the only place the code is compiled for Windows unless the
  maintainer runs it locally. Treat a red Windows job as blocking.
