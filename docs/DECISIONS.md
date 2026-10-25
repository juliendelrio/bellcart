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

**D27. `openai-chat` asks for one enum per question, talks only to the
configured endpoint, and every answer goes to a local log.**
The backend sends the signature with a strict JSON schema whose
properties are the questions, each an `enum` of its options, its levels,
or `yes`/`no`/`unsure` (mapped to 1, 0 and 0.5, which are not calibrated
probabilities). A value outside the enum is dropped as no opinion. The
backend declares itself uncalibrated, so its `leave` is never applied on
its own (D25). Requests go only to the configured URL: no redirect is
followed and no proxy is taken from the environment; plain `http` is only
accepted for this machine, so a key and signatures never cross a network
in clear. The key comes from `BELLCART_ARBITER_KEY`, never from the command
line where it would stay in the shell history. `bellcart arbitrate` prints
the exact first request and sends nothing without a yes; `--preview`
prints every signature and sends nothing. Each answer is appended to a
JSON Lines log with the signature as sent, the question set version, the
backend, the model, the answers and the verdict; corrections are later
lines of the same log. The log is outside the scanned profile like every
output, and a line cut short by a crash is skipped on reading.

**D28. The labelled set is built with `bellcart label` and keeps both
forms of each signature; `eval` fails when a folder to keep would be
left.**
Hand labelling is the evaluation set of every later arbiter decision, so
it gets its own command: each folder to review is shown, largest first,
and the answer is appended at once, so labelling can stop and resume. A
label stores the full and the redacted signature, so one set evaluates a
backend in either privacy mode. `eval` compares the backend's suggestion
(a held `leave` counts as `leave`) with the label and reports agreement
per decision. The figure that matters is separate: any folder labelled
`migrate` or `archive` whose verdict would apply `leave`. It must be zero,
and `eval` exits with an error otherwise, so a change of backend or of
the combination rules can be guarded in a script. A suggested `leave` on
a folder labelled `migrate` is listed too: held for the user, it still
measures how far a backend can be trusted.

**D29. The review interface is a page served on 127.0.0.1 by `tiny_http`,
written without a framework, reachable only with a token, and the user's
decisions are a plan source of their own (version 5).**
`bellcart ui` listens on 127.0.0.1 only and opens the default browser
through `rundll32 url.dll` on Windows, with no shell. The page, its script
and its style are embedded in the binary, so it works offline and loads
nothing from elsewhere. Two checks keep other pages away from the API: a
random 128-bit token the page gets in the URL fragment, which the browser
never sends to any server, and a `Host` header that must name 127.0.0.1,
which stops a site that points its own domain at this machine (DNS
rebinding). Every response carries a strict Content-Security-Policy. The
script is plain JavaScript and never puts scan data in the page as HTML:
folder names are personal and may hold anything. The review screen shows
the three carts with their totals, the units to decide one card at a
time, largest first and those without an arbiter opinion before those
with one (size times uncertainty), and the settled decisions folded, each
with its reason and a way to change it. A decision there is
`Source::User`, which may override a rule, a recipe or an arbiter,
`leave` included: it is the user's data. Hence plan version 5. The export
writes the plan, the notebook in HTML and Markdown, and the user's
decisions and arbiter answers to the decision log, in a folder outside
the scanned profile.

**D30. A decision taken from a parent folder covers every node at or
under it, except what a rule or a recipe settled.**
On a review card, each parent in the path can be chosen; migrate, archive
or leave then apply to that folder and all it contains. The plan may have
no node for the parent (a cut moves units up, D15 and D19), so the decision goes
to every node whose path lies under it, the user's earlier choices and the
arbiter's included: the newest choice of the user wins. Nodes settled by a
rule or a recipe keep their decision, as in "take this folder, but not
those", and the card says how many before the user decides. Files that sit
directly in the parent may belong to a rest higher up; that rest stays a
card of its own. Each changed node is logged as a correction.

**D31. A synced folder is recognised by a `review` rule and judged as one
unit; no rule leaves it.**
A folder a sync client keeps in step with a server (Synology Drive here)
is mostly stored there too, but files waiting to upload and files the sync
rules leave out exist only on this PC. Leaving it by rule would risk the
worst bug, and cutting it into sub-folders hides what it is. So a rule may
now take `decision: review`: it settles nothing, but makes the folder one
unit, whatever its size, with the rule's reason on the card (plan version
6, `UnitReason::Rule`). Such a unit is never folded into a larger one, and
rules still apply inside it. The client's recipe adds the checks to make
before leaving it: sync up to date, sync rules, backup tasks. Google Drive
in streamed mode keeps its files in the DriveFS cache, which the
`google-drive-cache` rule leaves after the recipe has the user check the sync;
its mirror folder and OneDrive should become such units once their markers
are checked on real machines (#56).

**D32. The profile's AppData is one unit to review.**
On a real profile, 27 of 47 cards were application folders under AppData.
Most of AppData comes back when the applications are reinstalled, and the
recipes settle what does not; caches stay left by their rules. But it can
hold the only copy of real data (mail stores, game saves, virtual machine
disks), so a `review` rule (D31) makes it one card rather than deciding
it. It is recognised by `AppData` holding `Local`, `LocalLow` or
`Roaming`. A recognised folder with nothing left to review once its
rules and recipes are applied is no card at all.

**D33. The review interface saves each decision as it is made, outside
the profile, and the next session replays them on a fresh scan.**
Closing the page lost every decision. Each decision is now written at once
to `bellcart-session.json` in the proposed export folder, which is checked
to be outside the scanned profile (when it is not, nothing is saved and
the page says so). The file holds the decisions in order, not the scan:
scanning again takes about a second, and replaying the decisions on a
fresh scan keeps the plan true to the disk. A decision whose folder is gone
is skipped. The first screen offers to resume, which Enter does, or to
start over; starting over replaces the file at the first new decision, not
before, so a mistaken click loses nothing.

**D34. AppData is cut into one card per application, and known
applications come with a proposed decision. This replaces D32.**
One card for all of AppData was not clear. `AppData/Local`, `LocalLow` and
`Roaming` are now cut one level (`parts`): each application folder is one
unit, never cut further, so a card is about one application; those under
`min_unit` share one card per folder, which lists their names. A pack
catalogue (`apps.yaml`) recognises known applications, also inside vendor
folders (`Google/Chrome`, `Microsoft/Edge`): each has its own card, even a
small one, titled with its name, with a proposed decision and its reason.
The user takes it with Enter or picks another; a proposal is never applied
on its own, so a wrong one costs a keystroke, not data. Even so, `leave` is
proposed only where the data is elsewhere for sure (cloud account,
download, cache), and mail or virtual disks get migrate or archive. Such
containers and their parts are never folded into a unit above, and a
folder with nothing left to review is no card.

## 2026-10-25

**D35. Fair-code licensing on the n8n model, in one repository. This
replaces P1.**
The code (engine, command line, interface, translations) is under the
Bellcart Sustainable Use License 1.0, adapted from n8n's Sustainable Use
License (itself derived from the Elastic License 2.0): free for personal
and non-commercial use and for use inside one's own organisation, and
redistribution only free of charge. Technicians may use it to migrate
their customers' computers for a fee, as n8n allows consulting: the tool is
in their hands, it is not what they sell. Reselling or bundling Bellcart,
a managed or fleet offer, and enterprise components need a commercial
agreement; a reseller licence is one of them. Enterprise components are
files with `.ee.` in their name or under an `ee` directory, under the
Bellcart Enterprise License; there are none yet, and enterprise mode (#35)
is where the paid features will be designed. Packs stay Apache-2.0 so
rules and recipes flow freely, and each pack declares its own licence.
Contributors to the code grant the maintainer the right to relicense their
work, which keeps the dual licensing possible; pack contributions stay
under the pack's licence. One repository is enough while the enterprise
code may be readable: `.ee` files are visible but not licensed for
production use, as at n8n. A separate private repository becomes useful
only if enterprise code must stay closed, or once licence keys exist.
Bellcart is not open source in the OSI sense and must not be described so.
Apache-2.0 alone (P1) was set aside because it allows anyone to resell the
tool, which leaves no room for the enterprise and reseller offers. The
Functional Source License, which turns each version into Apache-2.0 after
two years, was the other candidate; the n8n model was preferred for its
simpler, lasting split between free and paid uses. Not legal advice: a
lawyer should read the texts before the first commercial agreement.

**D36. Folders that hold many applications are cut like AppData, and
Windows components may be proposed to leave.**
`AppData/Local/Microsoft` and `AppData/Local/Packages` (Microsoft Store
application data) hold many applications each. Judged as one unknown
application, they made a card nobody could decide. They are now cut into
parts like AppData: a large application has its own card, small ones
share the folder's card. Known parts come with a proposal: WinGet and the
Teams meeting add-in are reinstalled, and the Windows shell's data
(`MicrosoftWindows.Client.CBS_*`) is rebuilt by Windows, so
`system-managed` joins the reasons allowed to propose leave. On the
developer profile this adds five cards, each now named, and moves no byte
between carts.

**D37. Dot folders at the profile root are judged whole, one card per
tool, and small ones join the profile's card.**
`.cache`, `.cargo`, `.local`, `.vscode` and the like are tool data, as
AppData is, but were cut by size into cards that meant little. A new rule
field, `fold_small`, makes a folder one unit judged whole, like a part,
which joins the card above when it is under `min_unit`: a dozen tiny dot
folders would otherwise make a dozen cards. Only the root is concerned
(`path_glob: ".*"`); a `.git` inside a project is cut as before. Rules
inside still apply, and the rules for keys come first, so `.ssh` stays
migrated. Known dot folders get a card with a proposal: `.vscode`
(extensions, brought back by the list the recipe saves), `.cargo/bin` and
`.local/bin` (installed tools), `.cache` (by convention, caches) to leave;
`.docker` and `.rustup` settings to migrate. The plan format is unchanged:
these units are parts.

**D38. A summary step comes between review and export.**
After dozens of cards, the user had no single view of what the plan will
do before it is written. The check step shows the carts as one bar, the
points to check, and the five largest folders of each cart with their
reason. The points come in order of what can cost data: folders still
undecided (they stay `review`, nothing is done for them), folders a person
or the arbiter chose to leave, folders a rule left inside the personal
folders (Desktop, Documents, Pictures, Videos, Music, by their name on
disk), and folders that could not be read. Each line leads back to its
card or to its row in the settled list. The step reads the review model
the page already has: it adds no API and no decision logic.

**D39. Installed programs are a list in the plan, apart from the folders.**
A program found installed (a folder in `AppData/Local/Programs`, a known
program folder such as Slack's) is something to install again, not
something to copy: its files come back with the installer. Folder
decisions cannot say "install this again", and the reinstall question
would otherwise be spread over many nodes. Plan version 7 adds
`programs`: one entry per program, with its folder, its name, the rule
that found it, and the user's choice (`undecided`, `yes`, `no`). Rules
mark such folders with `program: folder` or `program: each`; the engine
does not infer programs from a reason key, since reasons are only
translation keys. Setting the choice never changes a folder decision.
The same list will take the program folders of a drive scan (#75).

**D40. A whole drive can be scanned; the profile stays the reference.**
Data a person keeps outside the profile (`C:\dev`, a folder at the drive
root) was never in the plan, which is worse than a wrong `leave`: nobody
is asked about it. A scan of a drive root now locates the profile of the
account running Bellcart (`Users/<name>`) and records it in the plan
(version 8). Rules see paths relative to the profile inside it, so the
existing rules and recipes keep their meaning; outside it they see paths
relative to the drive, so rules written for any depth (`**/node_modules`)
also settle projects at the drive root. New `scope: drive` rules name what
Windows and installers keep outside the profile (`Windows`, `Program
Files`, `ProgramData`, the page file, through rules on `files`) and never
apply inside the profile or in a profile scan. `Program Files` produces no
cards: its programs join the reinstall list (D39), Windows components
excepted. Anything else at the drive root stays to review. Other profiles
are cards like any folder until elevation (#24) gives them their own
treatment, and the profile is found by account name until the Windows
crate resolves it by identifier (#22). Bellcart still never writes in the
tree it scans: exports of a drive scan go to another drive.


**D41. Each other profile on a drive is its own card.**
On a drive scan, the profiles of other accounts under `Users` used to fall
into the card for the rest of the drive root, next to driver installers
and leftovers. Deciding `leave` on that card would have left someone
else's documents. Each `Users/<name>` outside the scanned profile is now a
review card of its own, named as another account's profile; `Users/Public`
gets its own card as well. Cache rules still apply inside them. This is a
floor, not the treatment of other profiles that #24 will bring.

**D42. A drive scan may export to Bellcart's own folder on that drive.**
D40 sent the exports of a drive scan to another drive, but a PC often has
only one, or only a cloud drive (where the plan would be uploaded). The
maintainer chose: an external medium first, a dedicated local folder when
there is none. On a scan of a drive root, Bellcart may create
`bellcart-export` at that root and write its plan, notebook, decisions
and saved session directly in it; anything else in the scanned tree stays
refused, and a link of that name leading elsewhere is not the folder. The
interface proposes it only when Bellcart does not run from another drive,
and then says that a USB drive is better and a cloud drive is not. A
`scope: drive` rule migrates the folder, since its files are needed on the
new PC. Detecting removable and cloud drives needs the Windows crate (M5).

**D43. A folder that cannot be read can still be left by its rule.**
Without administrator rights, Windows refuses to list folders such as
`Recovery` or `Config.Msi` at the drive root, and `leave` rules could not
see them: they only added to the count of unreadable folders, and the
check step advised running as administrator for nothing. A folder that
cannot be listed is now settled by the first rule matching its path and
name, when that rule leaves it and needs no look inside (`contains`);
otherwise it stays counted where it was, and a recipe there keeps it
unsettled. The advice to run as administrator counts only unreadable
entries outside left folders, since reading those would change nothing.
