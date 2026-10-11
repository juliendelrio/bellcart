# English is the reference language: every key must exist here.

decision-migrate = Migrate
decision-archive = Archive
decision-leave = Leave behind
decision-review = To review

plan-title = Plan for { $root }
plan-settled = Settled by rules and recipes
plan-nothing-settled = No rule matched anything here.
plan-unreadable = { $count ->
    [one] { $count } folder could not be read.
   *[other] { $count } folders could not be read.
} Run as administrator to include them.
plan-to-review = To review, folder by folder
plan-applications = Applications found: { $names }
source-recipe = From the { $name } recipe
source-arbiter = Answers of the { $backend } arbiter
scan-progress = Scanning: { $folders } folders, { $size } read
plan-files ={ $count ->
    [one] { $count } file
   *[other] { $count } files
}

packs-title = { $name } { $version }
packs-rules = { $count ->
    [one] { $count } rule
   *[other] { $count } rules
}
packs-recipes = { $count ->
    [one] { $count } recipe
   *[other] { $count } recipes
}
packs-questions = { $count ->
    [one] { $count } arbiter question
   *[other] { $count } arbiter questions
}

# Reasons used by rules. A rule with `reason: foo` needs `reason-foo` here.
reason-regenerable-dependencies = Dependencies a package manager reinstalls
reason-build-output = Build output, recreated by the next build
reason-package-cache = Package manager cache, downloaded again on demand
reason-application-cache = Application cache, rebuilt automatically
reason-temporary-files = Temporary files
reason-system-managed = Managed by Windows
reason-keys-and-credentials = Keys and credentials: small, and painful to lose
reason-cloud-sync-cache = Local cache of a cloud drive, synced again by its client
reason-crash-reports = Crash reports, only of use to the application's developers
reason-installed-program = Installed program, to install again on the new PC
reason-update-download = Update already downloaded, fetched again when needed

# Why a folder to review is judged as one unit.
unit-small = small enough to judge as a whole
unit-homogeneous = mostly .{ $extension } files ({ $percent }%)
unit-homogeneous-no-extension = mostly files without an extension ({ $percent }%)
unit-depth-limit = deep folder, judged as a whole
unit-leaf = no sub-folder large enough to judge apart
unit-rest = files directly in this folder and its small sub-folders

notebook-title = Migration notebook for { $root }
notebook-intro = Tick each item as you go. Folders the plan migrates are copied with the rest: these items are what files alone do not carry.
notebook-empty = No application with a recipe was found on this profile.
notebook-before = Before you leave the old PC
notebook-after = On arrival on the new PC
notebook-copy = Take { $path }
notebook-copy-note = Take { $path }: { $note }
notebook-export-winget = Save the list of applications installed with winget (bellcart export can run it for you): { $command }
notebook-export-vscode = Save the list of Visual Studio Code extensions (bellcart export can run it for you): { $command }
notebook-export-registry = Save the registry key { $key } (bellcart export can run it for you): { $command }
notebook-install = Install it with winget: { $command }

export-intro = Bellcart runs each command below only if you answer yes. The files go to { $folder }.
export-none = Nothing to export: no application found on this profile has an export step.
export-winget = Applications installed with winget
export-vscode = Visual Studio Code extensions
export-registry = Registry key { $key }
export-ask = Run it? [y/N]
export-skipped = Skipped.
export-done = Saved: { $file }
export-failed = Failed: { $error }
export-missing = { $program } was not found on this PC.

arbitrate-none = Nothing to put to an arbiter: no folder is left to review.
arbitrate-preview = This is the exact request for the first folder. The others differ only by their summary.
arbitrate-confirm = Send { $count } folder summaries to { $url }? [y/N]
arbitrate-nothing-sent = Nothing was sent.
arbitrate-failed = { $path }: { $error }
arbitrate-stopped = Stopped after { $count } failures in a row: is the endpoint up?
arbitrate-leave-to-confirm = the arbiter suggests leaving it: to confirm

label-prompt = m (migrate), a (archive), l (leave), s (skip), q (quit)?
label-already = { $count } folders already labelled are skipped.
label-done = { $count } new labels in { $file }.
eval-confirm = Send { $count } labelled summaries to { $url }? [y/N]
eval-title = { $backend } on { $count } labelled folders: { $agreed } agreed
eval-label = { $label }: { $agreed } of { $total } agreed
eval-lost-none = No folder to keep would be left behind.
eval-lost = Folders to keep that would be left behind: { $paths }
eval-leave-suggested = Leave suggested, and held for the user, on folders to migrate: { $paths }
eval-failed = Without an answer: { $paths }
