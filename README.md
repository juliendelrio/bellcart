# Bellcart

**Pack your old PC. Leave the junk behind.**

[Français](README.fr.md)

When you change computers, you never know what to keep, so you keep
everything. Bellcart scans your old PC and tells you what is worth moving,
what can go to cold storage, and what can stay behind. Like a hotel bellhop,
it loads the cart for you. It does not carry it: the result is a plan you
review, then hand to the copy tool of your choice.

> **Status: pre-alpha.** The scan and the rule engine work. Recipes, the
> model arbiter and the review interface are specified but not built yet.
> See [what works today](#what-works-today).

## What it does

Every folder ends up on one of three carts:

| Cart | Meaning |
| --- | --- |
| **Migrate** | You need it on the new PC. |
| **Archive** | Worth keeping, not needed day to day. Goes to cold storage. |
| **Leave** | Can be re-downloaded or rebuilt, or is obsolete. |

Anything Bellcart is not sure about is marked **to review** and put in
front of you. Nothing is ever deleted or modified on the scanned machine.

It also looks after what you would forget: SSH keys, browser profiles,
editor settings, the list of installed applications. Some of these are not
files you can copy. Chrome passwords, for example, are tied to your Windows
account, so the right step is "turn on sync", not "copy this folder".
Bellcart knows the difference through community-maintained **recipes**.

## How it decides

1. **Scan.** Folder names, sizes, extensions and dates. File contents are
   never read.
2. **Rules.** Most decisions are obvious (`node_modules`, caches, temporary
   files) and need no model.
3. **Arbiter (optional).** For the remaining folders, a compact signature
   can be put to a model through typed questions. Bellcart works without
   any model: undecided folders are then simply left for you to review.
4. **Review.** You confirm or correct. A folder is never left behind on the
   word of an uncalibrated model alone.

Details are in the [V1 specification](docs/SPEC-V1.md).

## What works today

```console
$ bellcart scan C:\Users\me
Plan for C:\Users\me

  Migrate              2 B  1 file
  Archive              0 B  0 files
  Leave behind      7.1 MB  3 files
  To review         1.4 MB  3 files

Applications found: OpenSSH keys

Settled by rules and recipes
      3.8 MB  Leave behind  dev/app/node_modules  (Dependencies a package manager reinstalls)
      2.4 MB  Leave behind  AppData/Local/Google/Chrome/User Data/Default/Cache  (Application cache, rebuilt automatically)
    878.9 KB  Leave behind  AppData/Local/Temp  (Temporary files)
         2 B  Migrate       .ssh  (Keys and credentials: small, and painful to lose)

To review, folder by folder
      1.4 MB  .  (small enough to judge as a whole)
```

- `bellcart ui` opens the review interface in your browser: choose the
  profile, analyse it, decide folder by folder with three carts and
  their totals (click a parent in a folder's path to decide from there,
  with all it contains), correct what rules settled, check a summary of
  what goes where and what deserves a second look, then export the plan and
  the notebook. Each decision is saved as you make it, and the next
  session offers to resume. It runs on this machine only. Add `--endpoint` and
  `--model` to offer an arbiter there too.
- `bellcart scan <folder>` prints the summary above; `--json` prints the
  full plan, `--out plan.json` saves it (prefer it to `> plan.json`:
  Windows PowerShell 5.1 re-encodes what it redirects and garbles accented
  folder names), and `--signatures` prints what
  an arbiter would read about each folder to review (`--privacy redacted`
  masks names under personal folders, as a cloud arbiter would get
  them). Give a drive root (`C:\`) to scan the whole drive: rules settle
  Windows and the installed programs, which join the list of programs to
  install again, and your profile is judged as in a profile scan, so
  folders kept outside it (`C:\dev`) are in the plan too. The scan lists
  several folders at once; `--jobs 1` lists one at a time, which can be
  faster on a spinning hard disk.
- `--notebook notebook.html` writes the migration notebook: what to do
  by hand before leaving the old PC and on arrival, for each application
  found. A name ending in `.html` gives a page with checkboxes, any other
  gives Markdown. Bellcart refuses to write any file inside the folder it
  scans.
- `bellcart export <profile> --to <folder>` runs the exports the recipes
  ask for (the winget list of applications, VS Code extensions, registry
  keys). Each command is shown in full and runs only if you answer yes.
- `bellcart arbitrate <profile> --endpoint <url> --model <name>` puts the
  folders to review to a model behind an OpenAI-compatible endpoint (a
  local server or a cloud API; the key, if any, is read from
  `BELLCART_ARBITER_KEY`). It shows the exact request and asks before
  sending anything, masks personal names by default (`--privacy full`
  to send them), never applies a "leave" on the model's word alone, and
  logs every answer to `bellcart-decisions.jsonl`.
- `bellcart label <profile> --out labels.jsonl` asks you, folder by
  folder, what you would do with it, and `bellcart eval --labels
  labels.jsonl --endpoint <url> --model <name>` replays those answers
  against a model: agreement per decision, and above all any folder to
  keep that would be left behind.
- `bellcart packs` lists the built-in rules and recipes. `--pack <folder>`
  loads another pack next to it, for pack authors (see
  [docs/PACKS.md](docs/PACKS.md)).
- `--lang fr` switches the interface to French. The system language is
  used by default.

The plan format is described by [`schemas/plan.schema.json`](schemas/plan.schema.json).

## Build

Bellcart is written in Rust and targets Windows 10 and 11. The engine has
no Windows-only code, so you can build and test on Linux or macOS too.

```console
cargo build --release
cargo test
```

The release binary is a single portable executable: no installer, no
runtime, and no administrator rights needed for the current user's files.

## Privacy commitments

- No network access unless you configure an arbiter yourself.
- File contents are never read, only directory listings.
- Once the arbiter exists, it will show exactly what would be sent before
  anything leaves the machine.

## Contribute a recipe

Bellcart is only as good as what it knows about applications, and that
knowledge lives in plain YAML files under [`packs/`](packs/core), not in
code. If you know where an application keeps its data and what it takes to
move it, you can add it in a few lines. Start with the
[pack guide](docs/PACKS.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentation

- [V1 specification](docs/SPEC-V1.md)
- [Roadmap](docs/ROADMAP.md)
- [Decision log](docs/DECISIONS.md)
- [Pack format](docs/PACKS.md)

## Licence

Bellcart is fair-code, after the model of n8n: the source is open to read,
and the tool is free for personal use, for use inside your own
organisation, and for technicians who use it to migrate their customers'
computers. Selling or bundling Bellcart, offering it as a managed service,
and the future enterprise features need a commercial agreement, including
a reseller licence. It is not open source in the OSI sense.

- Engine, command line and interface:
  [Bellcart Sustainable Use License](LICENSE.md).
- Enterprise components (`.ee.` files, none yet):
  [Bellcart Enterprise License](LICENSE_EE.md).
- Packs, including the core pack: [Apache-2.0](packs/LICENSE), so rules
  and recipes can be reused freely.
