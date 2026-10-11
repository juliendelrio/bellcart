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

## 2026-10-11

**D18. The scan is parallel with `rayon`, and reports progress as
counters.**
A profile is a few very unequal trees (`AppData` alone can weigh as much
as everything else), so threads must take work from each other as they
run out: `rayon` does that work stealing, and is the standard Rust crate
for it. Writing it on `std::thread::scope` would save a dependency at the
cost of concurrency code of our own. Results are collected back in listing
order, so the plan does not depend on the number of threads, which a test
checks. The engine does not choose the pool: it runs in whatever pool the
caller installs, and the CLI sets its size with `--jobs`, since parallel
reads can be slower than sequential ones on a spinning disk. Progress is
three atomic counters (folders listed, files seen, bytes) that the scan
adds to without a lock; the engine still prints nothing, and the CLI draws
one line on stderr only when it is a terminal. On the maintainer's
profile, warm cache, 12 threads: 0.75 s instead of 2.8 s.

**D19. A rest smaller than `min_unit` joins the rest of the folder above,
and long digit suffixes are not extensions.**
A folder that is cut keeps its own files as a "rest" unit. Often that rest
is a `desktop.ini` or a few kilobytes next to large sub-folders, and it
made units no one would judge apart. Such a rest now joins the rest of the
folder above, its names prefixed with their folder in the signature, the
way sub-folders under `min_unit` already did. A rest that holds an
unreadable folder stays where it is, to show where access failed. A
suffix of four digits or more (`app.exe.old.33452`, `report.2024`) is a
counter, a process number or a year, and each value counted as an
extension of its own; three digits stay, for split archive parts. The
default budget is kept: on the maintainer's Windows 11 profile it gives
76 units instead of 91, the smallest 105 MB, all with a reason that reads
well. Other profiles may move the defaults again.

**D20. Rules from a first real profile: application caches, downloaded
updates and per-user programs are left.**
The rules added after the first run on a real Windows 11 profile match
folders an application owns and rebuilds: Chromium and Electron caches,
browser components, crash reports, VS Code caches, package stores
(pnpm, cargo, rustup, Hugging Face). Where the folder name is a common
word (`Cache`, `packages`, `*-updater`), the rule also requires the entry
the application leaves inside (`Cache_Data`, `RELEASES`, `installer.exe`).
`AppData/Local/Programs` is left as a whole: per-user installers put
programs there, and settings and documents live elsewhere; reinstalling is
the recipes' job. Site data that may be the user's own (`IndexedDB`,
`File System`) and WSL or Docker disk images stay to review. On that
profile the review list went from 76 units to 49.

**D21. Recipe steps settle their exact path, the keeping decision wins,
and the plan moves to version 3.**
A detected application's `copy` and `skip` steps become plan nodes with a
`recipe` source, so the plan says why a file or folder is migrated. A step
settles its exact path wherever it lies, including inside a folder a rule
settled: a recipe that needs one sub-folder of `AppData/Local/Programs`
can take it without the rule being weakened. When a rule and a recipe
settle the same path, the decision that keeps more wins, and the rule on a
tie because it carries a reason the user can read; when two recipes
disagree, `copy` wins. So a recipe can only rescue data from `leave`,
never send data there against a rule. Its own `skip` is limited at load
time to paths inside `%APPDATA%` or `%LOCALAPPDATA%`, below an application
folder, which keeps the user's folders out of a recipe's reach. Variables
are resolved against the scanned profile (the scan root), because an
elevated run may be another account; redirected known folders wait for the
Windows crate. Recipe nodes may be single files (`.gitconfig`), which a
reader of version 2 could not expect, hence version 3. Versions 1 and 2
still parse.

**D22. The engine gathers the notebook's tasks, the front end words them,
and no output is written inside the scanned tree.**
`notebook()` returns, per application found, the tasks of its recipe
before leaving and on arrival, with the pack's own texts; the CLI words
the generated ones (copy, export, install) in the interface language and
writes Markdown or HTML, as it will for the web interface. Paths are shown
as recipes write them (`%APPDATA%\...`), which is what the user will type
on the new PC. Until the export primitives run from Bellcart, an export
task shows the command to run. A file Bellcart writes (`--out`,
`--notebook`) is refused when it falls inside the scanned folder, compared
on canonical paths: the read-only rule covers Bellcart's own outputs.

**D23. Packs loaded from disk come after the core pack and add to it.**
`--pack <folder>` lets pack authors try a pack with the same checks as the
core one. Its rules come after the core rules and the first match wins, so
a pack loaded from disk can settle more folders but cannot change what the
core pack decides: it cannot send `.ssh` to `leave`. Ids must be unique
across packs rather than overriding each other, so a mistake shows as an
error instead of a silent replacement. Questions stay those of the core
pack until an arbiter needs others.

**D24. Export primitives are fixed commands, built by the engine and run
by the front end after an explicit yes.**
Three primitives exist: `winget export`, `code --list-extensions` and
`reg export`. The engine builds each command line from the primitive alone
(program, arguments, output file in the folder the user chose) and never
runs it; `bellcart export` shows the full command and runs it only on an
explicit yes, an empty answer or the end of the input meaning no. No shell
is involved: `code`, a batch file on Windows, is started as `code.cmd`,
which Rust only allows with safe argument quoting. A registry key is
limited when the pack loads to `HKCU\` and plain characters, so it cannot
read as an option or reach the machine's settings. The export folder must
be outside the scanned profile and is not created otherwise. The commands
run as the current account: exporting for another profile waits for the
elevated case of milestone M5.

**D25. Arbiter answers are combined in code; an uncalibrated `leave` is
held for the user or archived; the plan moves to version 4.**
`verdict::combine` turns typed answers into a decision. `migrate` and
`archive` apply, since a mistake there costs space. A `leave` applies only
from a calibrated backend reporting at least 0.9 confidence, and only when
the other answers agree: rebuildable (`regenerable` at least 0.7), not
personal (`personal` at most 0.3), of little importance (level 1 or less).
A missing answer counts as disagreement. Otherwise, a `leave` the answers
support stays to review with the opinion shown, for the user to confirm
in one click, and a `leave` they contradict goes to `archive`, where the
data is kept. A test sweeps every combination of answers, absurd values
included, and finds no path from an uncalibrated backend to `leave`.
Explanations are typed reasons for templates, never generated prose. A
verdict only decides a review unit nothing else decided: never a rule, a
recipe or the user. The plan records it as `Source::Arbiter`, hence
version 4; earlier versions still parse.

**D26. Redaction masks every name outside application paths, keeping
only the standard folders of a profile.**
In redacted mode a signature keeps its sizes, counts, dates, extensions
and unit reason. Paths under `AppData` and the dot folders at the profile
root stay in clear: they name applications, which is what the arbiter
needs. Everything else is masked, including folders the user made at the
profile root, since a name like a family or a client is personal wherever
it sits; only the standard folders (`Documents`, `Pictures`...) keep their
name as the first part of a path. A masked name becomes `<n>`, a file
`<n>.ext`, numbered in order of appearance so the same name reads the same
within a signature and a signature stays the same from one scan to the
next. A folder a rule settled keeps its last name (`node_modules`), which
the rule matched and which says nothing about the person. Redacted is the
default for an arbiter; `scan --signatures --privacy redacted` shows what
would be sent.

## Proposed, to confirm

**P1. Licence: Apache-2.0** for code and for the core pack.
Permissive, with an explicit patent grant, and compatible with a future
commercial offer for enterprises. A copyleft licence would be the choice
if keeping derivatives open mattered more than adoption.
