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

## Proposed, to confirm

**P1. Licence: Apache-2.0** for code and for the core pack.
Permissive, with an explicit patent grant, and compatible with a future
commercial offer for enterprises. A copyleft licence would be the choice
if keeping derivatives open mattered more than adoption.
