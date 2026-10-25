# Pack format

A pack is a folder of YAML files. It contains knowledge, never code.

```text
packs/<id>/
  pack.yaml          metadata
  questions.yaml     arbiter question set (optional)
  rules/*.yaml       folder rules
  recipes/<id>.yaml  one application per file
```

Format version: `1`. The Rust types in `src/pack.rs` define it. The JSON
Schemas in `schemas/` are generated from those types; each YAML file in the
core pack starts with a `yaml-language-server` comment so editors validate
and autocomplete as you type.

Unknown fields are rejected. A typo fails loudly instead of being ignored.

## pack.yaml

```yaml
id: core
name: Bellcart core pack
version: 0.1.0     # version of the content
format: 1          # version of this format
description: Rules and recipes that ship inside the Bellcart binary.
license: Apache-2.0
```

## Rules

A rule settles a folder **and everything under it**. Rules are not applied
inside a folder another rule already settled. Rules are tried in file name
order, then in the order written; the first match wins.

```yaml
rules:
  - id: python-venv
    decision: leave              # migrate, archive or leave
    reason: regenerable-dependencies
    match:
      name_in: [venv, .venv, env]
      contains: [pyvenv.cfg]
```

| `match` field | Holds when |
| --- | --- |
| `name_in` | the folder name is one of these |
| `path_glob` | the path relative to the scan root matches |
| `contains` | the folder directly contains an entry with one of these names |

All the fields given must hold. Matching is case-insensitive. A rule needs
`name_in` or `path_glob`; `contains` alone is refused.

In `path_glob`, `/` separates folders, `*` stays inside one folder name,
`**` crosses folders, `{a,b}` is an alternative. Start patterns with `**/`
so they match whether the user scans one profile or a whole drive.

`reason` is a translation key: `locales/en/bellcart.ftl` and every other
language must define `reason-<key>`. Reuse an existing reason when one fits.

### Writing a safe `leave` rule

A wrong `leave` loses someone's data. Before adding one, ask: could a
folder with this name or path hold a person's own files?

- `node_modules` cannot. A name is enough.
- `venv`, `target`, `build`, `cache` can. Add `contains` with a marker file
  the tool itself creates, or anchor the rule with a `path_glob`.
- If you cannot make it unambiguous, do not write the rule. Undecided
  folders go to review, which is safe.

### Recognising a folder without deciding it: `review`

`decision: review` settles nothing. The folder becomes **one unit to
review**, whatever its size, with the rule's reason on its card, instead of
being cut into its sub-folders. Rules still apply inside it, and it is
never folded into a larger unit. Use it for folders Bellcart recognises but
must leave to the user, such as a folder a sync client keeps in step with a
server: most of it is also stored there, but not files waiting to upload
or left out by the sync rules.

```yaml
rules:
  - id: synology-drive-folder
    decision: review
    reason: synced-synology-drive
    match:
      path_glob: "**"                        # wherever the user put it
      contains: [.SynologyWorkingDirectory]  # the marker the client creates
```

A recognised folder with nothing left to review, once the rules and recipes
inside it are applied, is no card.

A `review` rule takes four more fields:

| Field | Effect |
| --- | --- |
| `name` | What the folder belongs to, shown as the card's title: an application's name. |
| `propose` | A decision shown on the card, which the user takes with Enter. It is never applied without the user. |
| `parts` | Cut the folder one level instead: each sub-folder no other rule knows is one unit, of the kind this translation key names (`reason-<key>`). Sub-folders smaller than `min_unit` share the folder's own card, which shows `reason`. |
| `fold_small` | `true`: the folder is judged whole, like a part, but one smaller than `min_unit` joins the card of the folder above instead of having its own. Without it, a recognised folder always has its own card. |

`parts` excludes `name` and `propose`: its parts are unknown. `fold_small`
excludes all three: it names a kind of folder, such as the dot folders at
the profile root (`root-dot-folders`), not a known one. The core pack
cuts `AppData/Local`, `LocalLow` and `Roaming` into one card per application
(`units.yaml`), and names the applications it knows, with a proposal
(`apps.yaml`):

```yaml
rules:
  - id: app-thunderbird
    decision: review
    name: Thunderbird
    propose: migrate
    reason: app-mail
    match:
      path_glob: "**/AppData/Roaming/Thunderbird"
```

Any rule may also say that a folder shows an installed program, with
`program`. The program then goes on the plan's list of programs to
install again, where the user says, for each one, whether it is wanted on
the new PC. It changes no folder decision: the program's files are left
or reviewed as the rule says, since the installer brings them back.

| `program` | Effect |
| --- | --- |
| `folder` | The folder itself is one program, named by the rule's `name`, or by the folder name. |
| `each` | Each sub-folder is one program, named by its folder name. |

```yaml
rules:
  - id: user-installed-programs
    decision: leave
    reason: installed-program
    program: each
    match:
      path_glob: "**/AppData/Local/Programs"
```

`program` does not go with `fold_small`. Mark only folders that are the
program itself, not its settings or its data: those have their own rules.

Propose `leave` only when the data is somewhere else for sure: a cloud
account, a download, a cache. When an application may keep the only copy
of something (mail, virtual disks), propose to migrate or archive it. A
test holds the reasons a `leave` proposal may give.

Pair it with a recipe for the client, whose `before` steps say what to
check before the user leaves the folder behind.

## Recipes

One file per application, named after its `id`.

```yaml
id: mozilla-firefox
name: Mozilla Firefox
detect:
  any:
    - path: '%APPDATA%\Mozilla\Firefox\Profiles'
before:                  # on the old PC
  - type: copy
    path: '%APPDATA%\Mozilla\Firefox'
    note:
      en: Profiles, with bookmarks, history and saved passwords.
      fr: Profils, avec favoris, historique et mots de passe enregistrés.
after:                   # on the new PC
  - type: install
    winget: Mozilla.Firefox
```

Step types. The list is closed: there is no step that runs a command.

| `type` | Fields | Effect |
| --- | --- | --- |
| `copy` | `path`, `note` (optional) | Migrate this file or folder. A missing path is ignored. |
| `skip` | `path` | Never take this, even if a parent is migrated. |
| `manual` | `text` | Something only the user can do. Goes to the notebook. |
| `export-winget-packages` | | Save the list of winget-installed applications. |
| `export-vscode-extensions` | | Save the list of VS Code extensions. |
| `export-registry-key` | `key` | Save one key of the user's registry (`HKCU\...`) to a `.reg` file. |
| `install` | `winget` | Install with winget, by package identifier. |

Paths start with `%USERPROFILE%`, `%APPDATA%` or `%LOCALAPPDATA%`, and
must not climb out with `..`: a pack that does not follow this does not
load. Variables are resolved against the scanned profile, not the account
Bellcart runs as. Write paths in single quotes so backslashes stay literal.

How steps meet rules:

- An application is found when any `detect` path exists. Only then do its
  `copy` and `skip` steps apply.
- A step settles exactly its path, a file or a folder, wherever it lies:
  in what is still to review, or inside a folder a rule settled. Rules are
  not applied below it, but other recipe steps are.
- When a rule settles the same path, the decision that keeps more wins
  (`migrate`, then `archive`, then `leave`), and the rule on a tie. A
  recipe can rescue a folder from a `leave` rule, never the reverse.
- When two recipes settle the same path, `copy` wins.
- A `skip` must point inside `%APPDATA%` or `%LOCALAPPDATA%`, below an
  application's own folder. A recipe cannot send the user's folders to
  `leave`.

Export steps run only through `bellcart export`, which shows each command
and runs it after the user says yes. A registry key must start with
`HKCU\` and keep to letters, digits, spaces and `_ - . \`; anything else
does not load.

`name` is one string when it is the same in every language (`Git`), or
`en` plus translations when it is not:

```yaml
name:
  en: OpenSSH keys
  fr: Clés OpenSSH
```

Texts (`text`, `note`) need `en`. Other languages are optional and fall
back to English. Recipes in the core pack must also provide `fr`.

### What makes a good recipe

- Say **why** in a comment when the obvious step is wrong. Chrome's saved
  passwords are encrypted for the Windows account, so the recipe asks for
  sync or an export instead of copying the profile.
- Prefer the application's own export or sync over copying internals.
- `skip` only what is regenerable for certain.
- Test it on a real installation, and say which version in the pull
  request.

A missing primitive is a reason to open an issue, not to work around it
in a `manual` text.

## Questions

`questions.yaml` holds what an arbiter is asked about an undecided folder.
The shapes follow the System One request format.

```yaml
version: 1
questions:
  regenerable:
    type: noul
    instructions: Can this content be re-downloaded or rebuilt from other sources?
```

| `type` | `criteria` |
| --- | --- |
| `choice` | map of option to description |
| `score` | list of levels, from low to high |
| `noul` | optional `"true"` and `"false"` descriptions |

Keep each question atomic. Ids are stable because decision logs refer to
them; bump `version` when a question changes meaning.

## Trying a pack

Load a pack folder next to the built-in one with `--pack`, as many times
as needed:

```sh
bellcart --pack ./my-pack packs
bellcart --pack ./my-pack scan C:\Users\me --notebook notebook.html
```

The folder goes through the same checks as the core pack. An error names
the folder, the file, the line and the field:

```text
bellcart: cannot load the pack in ./my-pack: invalid pack file rules/typo.yaml:
error: line 5 column 5: unknown field `mach`, expected one of id, decision, reason, match
```

Rules of a pack loaded with `--pack` come after the core rules. The first
match wins, so a pack can settle folders the core pack leaves to review,
but cannot change what a core rule decides. Pack, rule and recipe ids must
be unique across all packs. A `reason` that no locale defines is shown as
its key: reuse the reasons listed in `locales/en/bellcart.ftl`.

## Sources and licensing

Other projects publish databases of where applications keep their data.
They are useful to learn from, but do not copy entries from them unless
their licence allows it and you say so in the pull request. Write recipes
from the application's documentation and from what you observe on a real
machine.

Packs are not covered by the licence of the Bellcart code. A pack declares
its own licence in `pack.yaml` (an SPDX identifier); the core pack is
[Apache-2.0](../packs/LICENSE), and a contribution to it is published under
that licence.
