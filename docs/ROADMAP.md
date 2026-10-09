# Roadmap to V1

Each milestone is a usable step and ends with something to check on a real
Windows machine. They are sized to become one GitHub milestone each.

## M0. Skeleton (done)

Tree abstraction, scan, rule engine, pack loader, plan format, CLI in
English and French, generated schemas, CI.

## M1. A scan worth reading on a real profile

- Break the review mass into reviewable folders: descend until a folder is
  small enough or homogeneous enough to be judged as one unit, with a
  depth and size budget.
- Build the signature of each review folder (the arbiter's `state`), within
  the token budget, and expose it with `bellcart scan --signatures`.
- Parallel scan and a progress indicator.
- Run on real Windows 10 and Windows 11 profiles, fix the seed rules, add
  the obvious missing ones.
- Hand-label 200 to 300 signatures from real machines: this is the
  evaluation set for every later arbiter decision.

Check: on a real profile, the scan finishes in minutes and the review list
fits on a few screens.

## M2. Recipes and the migration notebook

- Expand `%VARIABLES%` and run recipe detection.
- Apply `copy` and `skip` steps to the plan.
- Generate the notebook: "before you leave" and "on arrival" checklists,
  in the user's language, as Markdown and HTML.
- Implement the export primitives, each shown and confirmed before it runs.
- Google Drive for desktop recipe. The `google-drive-cache` rule leaves
  DriveFS, which is only safe once every change has been uploaded: "before
  you leave" must ask the user to check that Drive shows it is up to date.
- `--pack <folder>` to load packs from disk, for pack authors.

Check: the notebook for a developer machine and for a family PC lists
nothing wrong and misses nothing obvious.

## M3. Arbiter

- `openai-chat` backend with structured output, configurable endpoint,
  model and key.
- Combination logic: typed answers to a decision, under the prudence rules.
- Full and redacted privacy modes, and the "what will be sent" preview.
- Local decision log, with user corrections.
- `bellcart eval`: replay the labelled set against a backend and report
  agreement, so backends can be compared on data.

Check: on the labelled set, the arbiter never turns a folder labelled
"migrate" into an applied "leave".

## M4. Review interface

- Local web server and embedded assets, opened in the default browser.
- The four-step assistant and the review screen: three carts, the
  "to decide" queue, folded settled decisions with their reasons.
- Export of the plan and the notebook.

Check: someone who has never seen the tool sorts a real profile without
help.

## M5. Windows crate and elevation

- Member crate for Windows API calls: known folders by identifier,
  elevation detection, relaunch elevated with the original profile passed
  on.
- Other user profiles and unreadable folders when elevated.

Check: an unelevated run reports what it could not read and the elevated
relaunch fills the gap.

## M6. Release

- Signed portable executable built in CI.
- Release notes, checksums, winget manifest.
- Reserve the crate name on crates.io as soon as a version is publishable.

## After V1

- `systemone` and `decisions` backends; a small model fine-tuned on the
  decision log.
- Destination and archive connectors.
- Per-language keyword lists for name-based rules.
- A pack registry.
- Enterprise mode: policy packs, unattended runs, fleet reports.
