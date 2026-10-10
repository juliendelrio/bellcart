# Decision log

One entry per decision that would be expensive to rediscover. Add to the
end. To reverse a decision, add a new entry that supersedes the old one
instead of editing history.

## 2026-10-09

**D1. V1 produces a plan, not a transfer.**
The value is in deciding what to move. Copying is a solved problem and
carries the risk. A plan that maps onto include and exclude lists lets any
copy tool do the rest.

**D2. Three destinations plus review.**
Migrate, archive, leave. People keep too much out of fear of losing
something. "Archive" gives them a safe place for what they are not sure
about, which is what lets them stop migrating everything. In V1 migrate
and archive each point to a local folder. Connectors to storage and backup
tools come later.

**D3. The model arbitrates, it does not scan.**
A disk holds hundreds of thousands of files and most decisions are
deterministic. Scan and rules run locally. Only compact signatures of
undecided folders are put to a model. No file content in V1.

**D4. Bellcart must work with no model at all.**
An individual has no model endpoint. Rules and recipes do most of the work;
the arbiter is a bonus when one is configured.

**D5. One portable executable, elevation on demand.**
Not two builds. The tool runs unelevated on the current profile, detects
its rights, and offers to relaunch elevated for advanced features.

**D6. Rust.**
Single static binary, direct Windows API access, fast parallel scan,
memory safety for a tool that reads personal data and may run elevated.
The project is developed with a coding agent: the compiler and `cargo`
tooling give it a tight verification loop. The cost is a higher barrier
for code contributors, accepted because most contributions will be packs.

**D7. CLI plus an embedded web UI on localhost.**
Engine as a library, two front ends. A browser tab instead of a WebView,
because the WebView runtime may be missing on the old machine being left.

**D8. Knowledge lives in declarative packs with closed primitives.**
Rules, recipes and questions are YAML. No code plugins in V1 and no shell
commands in recipes: the tool reads personal data, sometimes elevated, and
ships under the maintainer's signature. Executable steps are named
primitives implemented in the engine.

**D9. The arbiter contract follows the System One request format.**
`state` plus typed `questions` (choice, score, noul). Decision models and
gateway routes speak it natively, and a classic LLM can be constrained to
it. V1 ships the `none` and `openai-chat` backends. Decision-model backends
are prepared, not shipped: which one to plug in is decided later, on
measured results.

**D10. Asymmetric prudence.**
A wrong "leave" loses data, a wrong "migrate" wastes space. An uncalibrated
backend can never cause a folder to be left on its own.

**D11. English by default, French from day one.**
Internationalisation is built in from the first commit, and enforced by
tests, because retrofitting it is painful.

**D12. Name: Bellcart.**
The hotel luggage cart. It says the limit (not everything fits), reads in
English, and is short to type. "Valise" was the first choice but is taken
on crates.io and PyPI by an active Rust project.

**D13. YAML for packs, schemas generated from the Rust types.**
YAML is what contributors already know from winget manifests and CI files.
The Rust types are the single source of truth; JSON Schemas are generated
from them and checked in for editor support.

**D14. One package, engine as library and CLI as binary.**
Packs, locales and schemas sit at the repository root where contributors
find them, and are still inside the published crate. Windows API code will
come as a separate member crate so `unsafe` stays confined.

**D15. The review mass is cut by local thresholds, and the plan moves to
version 2.**
A review folder is one unit when it is small (`split_above`), homogeneous
(one extension holds most of the bytes and the others are small), at the
depth limit, or has no sub-folder of at least `min_unit`. Otherwise it is
cut. Sizes exclude folders already settled by a rule. The criteria are
local, so a folder's cut does not change when unrelated folders grow: a
signature stays the same from one scan to the next, which the labelled
set and the decision log rely on. A global budget (cut the largest units
until there are N) would bound the list but loses that stability. If real
profiles produce too many units, a ceiling that folds the smallest units
back can be added on top. The homogeneity test requires the minority to be
small in bytes, not only in share, so a mostly-video folder cannot hide
gigabytes of documents. The plan moves to version 2 because a `review`
node may now hold `review` nodes, which a reader of version 1 could not
expect. Version 1 plans still parse.

## 2026-10-10

**D16. Branches are named `<type>/<issue>-<short-description>`.**
Work is tracked in GitHub issues, one milestone per roadmap step. Putting
the issue number in the branch links the two without a tool. The types
follow the kinds of contribution this project gets, with `pack` apart
because pack changes are reviewed by different rules than code. CI rejects
a pull request whose branch does not follow the convention.

**D17. Signatures are fixed English text, built from samples taken during
the scan and kept out of the plan.**
A signature is input for a model and the evaluation set, so the same unit
must give the same text whatever the interface language: it is never
translated. The largest entries of each unit are sampled while the tree is
walked, which costs no extra disk access and gives sizes for free;
re-listing units afterwards would need a second walk to size sub-folders.
Samples travel next to the plan (`Scan::samples`), not inside it: the plan
goes to copy tools, which have no use for file names. The text stays under
3200 characters (about 800 tokens at four characters per token, without a
tokenizer dependency) by dropping names from the longest list first, then
extensions, always in the same order. The data (`Signature`) is kept apart
from the text so a redacted form can come later without changing what is
collected.

## Proposed, to confirm

**P1. Licence: Apache-2.0** for code and for the core pack.
Permissive, with an explicit patent grant, and compatible with a future
commercial offer for enterprises. A copyleft licence would be the choice
if keeping derivatives open mattered more than adoption.
