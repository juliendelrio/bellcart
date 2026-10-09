# Bellcart V1 specification

Status: draft, 2026-10-09. The reasons behind each choice are in the
[decision log](DECISIONS.md). The order of work is in the [roadmap](ROADMAP.md).

## 1. Problem and promise

When people replace a computer they do not know what to keep, so they keep
more than they need, and still forget what matters: keys, application
settings, things that are not plain files.

Bellcart's promise: **you move to the new PC with only what matters, and
nothing you will miss stays behind.**

The value is in the decision, not in the transfer. V1 produces a plan and
a checklist. It copies nothing.

## 2. Scope

In V1:

- Windows 10 and Windows 11, as a single portable executable.
- Works without administrator rights on the current user's profile, and
  offers to relaunch elevated for more (other profiles, locked files).
- Scans a folder tree chosen by the user.
- Sorts folders into migrate, archive, leave, or review.
- Detects known applications and lists what to do about them, before
  leaving the old PC and on arrival on the new one.
- Optional model arbiter for undecided folders, through an OpenAI-compatible
  endpoint the user configures.
- Command line and an embedded web interface served on localhost.
- English by default, French supported from the first release.
- Community packs: declarative files, no code.

Out of V1:

- Copying, moving or deleting anything.
- Reading file contents.
- Decision models (System One class). The contract is ready for them, the
  backends come after V1.
- Destination and archive connectors (cloud storage, NAS, backup tools).
  V1 targets two local folders named by the user.
- Fleet and enterprise features.
- macOS and Linux as scan targets. The engine builds and tests there.

## 3. Users

V1 is for an individual replacing their own PC, comfortable enough to
download a tool and follow a checklist. The enterprise case (a technician
migrating many machines, policy packs, unattended runs) is a later product
built on the same engine and the same packs.

## 4. Pipeline

```text
tree -> scan -> rules -> signatures -> arbiter -> review -> plan + notebook
                 ^                        ^
               packs                  questions (pack)
```

1. **Scan.** Walk the tree through a read-only abstraction. Collect, per
   folder: size, file count, bytes per extension, oldest and newest
   modification. Links and junctions are never followed. Files are never
   opened.
2. **Rules.** A rule settles a folder and everything under it. Rules are
   not applied inside a settled folder. First match wins.
3. **Signatures.** Each folder still undecided is summarised in a compact
   text of about 800 tokens at most: path, size, extension mix, date range,
   a sample of child names, the application it belongs to if a recipe
   detected one.
4. **Arbiter.** Optional. Answers typed questions about a signature.
5. **Review.** The user confirms, corrects, or decides what is left.
6. **Outputs.** The plan and the migration notebook.

The unit of decision is the folder, never the file.

## 5. Decisions

| Decision | Meaning | Who may take it |
| --- | --- | --- |
| `migrate` | Needed on the new PC | rule, recipe, arbiter, user |
| `archive` | Keep in cold storage | rule, arbiter, user |
| `leave` | Regenerable or obsolete | rule, user, calibrated arbiter |
| `review` | Not settled | default |

**Asymmetric prudence.** Keeping too much costs disk space. Leaving the
wrong folder costs data. So:

- A `leave` from an uncalibrated arbiter is never applied on its own. It
  becomes `archive`, or goes to review with the arbiter's opinion shown.
- A rule may only decide `leave` when the match cannot be a user folder.
  Ambiguous names need a `contains` condition (a `venv` folder is only a
  virtual environment if it holds `pyvenv.cfg`).
- When in doubt, `review`.

Every decision records who took it and why, so the interface can explain
it and the user can trust it.

## 6. Outputs

**The plan** is a JSON document ([schema](../schemas/plan.schema.json)).
It is a tree of exceptions: each node says "this folder gets this decision,
except these sub-folders". This is how copy tools already think (take this
folder, exclude those), so a plan translates directly into robocopy
arguments or any other tool's include and exclude lists.

**The migration notebook** is the human half: two checklists generated
from the recipes of detected applications.

- *Before you leave the old PC*: exports to run, sync to turn on, things
  to close, work to push.
- *On arrival*: what to install, where to put things back, what will ask
  you to sign in again.

## 7. Packs

All knowledge about folders and applications is data, in packs
([format](PACKS.md)). The core pack is embedded in the binary. A pack holds:

- **Rules**: folder patterns and their decision.
- **Recipes**: one per application. How to detect it, what to copy, what to
  skip, what to export, what the user must do by hand, how to restore.
- **Questions**: what the arbiter is asked.

Recipes use a closed list of step types. A recipe cannot carry a shell
command. Anything that executes (a winget export, a registry export) is a
named primitive implemented in the engine, shown to the user and confirmed
before it runs. This is what makes third-party packs safe to accept in a
tool that reads personal data, sometimes elevated.

Code plugins are out of V1. Destination and archive connectors will need
code and will come behind a dedicated interface.

## 8. Arbiter

### Contract

One internal contract, shaped after the System One request format:
a `state` (the signature), and named `questions` of three types.

| Type | Asks | Returns |
| --- | --- | --- |
| `choice` | pick one option | choice, probabilities, confidence |
| `score` | rate on an ordered scale | score, probabilities, confidence |
| `noul` | is this true? | probability of yes |

Questions are atomic and live in the pack, versioned. The engine combines
the answers in code. Explanations shown to the user are built from the
typed answers and translated templates, not from generated prose.

### Backends

| Backend | When | Notes |
| --- | --- | --- |
| `none` | V1 | Rules and recipes only. Bellcart must stay useful here. |
| `openai-chat` | V1 | `/v1/chat/completions` with structured output, one enum per question. Not calibrated. |
| `systemone` | after V1 | Sends the contract as is to a `/v1/systemone` endpoint. |
| `decisions` | after V1 | Gateway route `/v1/decisions`. Schema to confirm. |

Each backend declares whether its confidences are **calibrated**. An
uncalibrated backend reports no confidence rather than an invented one, and
falls under the prudence rules of section 5.

### Decision log

Every arbiter answer is logged locally with the signature, the question set
version, the backend, and the user's correction if any. The log never
leaves the machine unless the user exports it. It is the evaluation set for
comparing backends, and later the training material for a specialised
small model.

## 9. Privacy

- Default: nothing leaves the machine. No telemetry.
- Folder and file names are personal data. With an arbiter configured:
  - **Full mode** sends signatures as built. Meant for a local or trusted
    endpoint.
  - **Redacted mode** masks names under personal folders (Documents,
    Pictures, Desktop and the like) and keeps extensions, sizes and dates.
    Application paths are sent in clear. Meant for cloud endpoints.
- A preview shows exactly what would be sent, before the first request.
- File contents are never sent in V1, in any mode.

## 10. Interface

The engine is a library. Two front ends use it:

- **CLI** for scripting and for people who prefer it.
- **Embedded web UI** served on localhost and opened in the default
  browser. No WebView dependency, since the machine being left is often an
  outdated one.

The interface is a four-step assistant: choose, analyse, review, export.
The review screen is the product:

- three carts with running totals in GB;
- a "to decide" queue sorted by size times uncertainty, handled one card
  at a time;
- settled decisions folded away, each with its reason, open to correction.

It must never be a tree of half a million checkboxes.

## 11. Windows specifics

- **Known folders** are resolved by identifier, never by name: Downloads
  is `Téléchargements` on screen and `Downloads` on disk.
- **Links and junctions** are not followed. Legacy profile junctions loop.
- **Cloud placeholders** (OneDrive files on demand): only attributes are
  read, so nothing is downloaded by the scan.
- **Elevation**: the tool detects whether it is elevated and offers to
  relaunch. If the user is not an administrator, the elevated process runs
  as another account, so the original profile is passed explicitly.
- **One binary**, C runtime linked statically, no installer.
- All Windows API calls live in one dedicated crate. Everything else
  forbids `unsafe`.

## 12. Internationalisation

- Interface strings are Fluent files, one folder per language. English is
  the reference and the fallback.
- Rule reasons are translation keys. Recipe texts carry English and
  optional translations inline, so a pack is self-contained.
- Tests fail if French misses an English key, or if a core recipe text has
  no French.
- Rules must match folder names in any Windows language. Name-based
  keyword lists per language are a later addition.

## 13. Distribution

- Portable `.exe` on GitHub Releases, built in CI.
- **Code signing is required for V1**: an unsigned executable is stopped
  by SmartScreen and often flagged by antivirus. Options to evaluate:
  a signing programme for open source projects, a cloud signing service,
  or a classic certificate. Eligibility and cost to be checked.
- Publishing to winget once signed.

## 14. Quality

- The scan and rule behaviour is specified by tests on in-memory trees
  (`tests/scan.rs`). A behaviour change starts with a test change.
- CI runs on Windows Server 2022 and 2025 runners (the Windows 10 and
  Windows 11 code bases), and on Linux.
- `clippy` denies `unwrap`, `expect` and `panic` outside tests.
- JSON Schemas are generated from the Rust types and checked in CI, so the
  documented format cannot drift from the code.

## 15. Where the code stands

| Part | State |
| --- | --- |
| Tree abstraction, in-memory tree for tests | done |
| Scan, per-folder stats, plan as tree of exceptions | done |
| Rule engine and pack loader | done |
| Core pack: seed rules, 7 recipes, question set | done, to verify on real machines |
| CLI `scan` and `packs`, English and French | done |
| JSON Schemas generated from types | done |
| Arbiter contract types, `none` backend | done |
| Breaking the review mass into reviewable folders | to build |
| Recipe detection and migration notebook | to build |
| `openai-chat` backend, decision log, privacy modes | to build |
| Web interface | to build |
| Windows crate: known folders, elevation | to build |
| Signed release pipeline | to build |

## 16. Open questions and risks

1. **Who provides the model?** An individual has no endpoint. Bellcart
   must be convincing with `none`. This is the riskiest assumption.
2. **Do small decision models read French folder names well?** Unknown.
   To measure on a few hundred hand-labelled signatures before choosing.
3. **Gateway `/v1/decisions` schema** has not been read in full. Confirm
   before writing that backend.
4. **Windows ships its own PC-to-PC transfer.** It moves everything
   without sorting, and not applications or passwords. Bellcart's place is
   the decision and the invisible. Keep the positioning sharp.
5. **Name.** No conflicting software was found for "Bellcart". The official
   trademark registers were not searched.
6. **Signing cost and eligibility** for an individual maintainer.
7. **Seed rules and recipes** were written from documentation, not yet
   checked against real Windows 10 and 11 profiles.
