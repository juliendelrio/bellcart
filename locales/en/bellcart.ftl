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
source-user = Your decision
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

ui-listening = Bellcart is open in your browser: { $url } (Ctrl+C here to stop it)
ui-not-a-folder = { $path } is not a folder.
ui-title = Bellcart
ui-step-choose = Choose
ui-step-analyse = Analyse
ui-step-review = Review
ui-step-export = Export
ui-choose-title = Which profile do you want to sort?
ui-choose-help = Bellcart only reads. It never moves, changes or deletes anything on this PC, and nothing leaves it unless you ask.
ui-choose-label = Folder to analyse
ui-choose-start = Analyse
ui-scanning-title = Analysing
ui-scanning-help = Bellcart lists the folders and their sizes. It does not open files.
ui-failed = The analysis failed: { $error }
ui-error = Something went wrong: { $error }
ui-carts = Carts
ui-to-decide = To decide
ui-folder-one = { $count } folder
ui-folder-other = { $count } folders
ui-file-one = { $count } file
ui-file-other = { $count } files
ui-unreadable-one = { $count } folder could not be read
ui-unreadable-other = { $count } folders could not be read
ui-review-title = Review
ui-review-help = Decide folder by folder, largest first. What rules and recipes already settled is folded below, open to correction.
ui-applications = Applications found: { $names }
ui-card-position = Folder { $index } of { $count }
ui-card-modified = modified { $oldest } to { $newest }
ui-card-kinds = Kinds of files
ui-card-folders = Largest folders
ui-card-files = Largest files
ui-card-settled = Already settled inside
ui-card-opinion = The arbiter suggests: { $decision }
ui-card-opinion-leave = The arbiter suggests leaving this folder behind. Bellcart never leaves a folder on a model's word: decide it yourself.
ui-no-extension = no extension
ui-more = { $count } more
ui-none = None
ui-action-migrate = Migrate
ui-action-archive = Archive
ui-action-leave = Leave
ui-action-later = Later
ui-decided = { $path }: { $decision }
ui-queue-empty = Everything is decided.
ui-queue-empty-help = You can still change any decision below, then export the plan and the notebook.
ui-go-export = Go to export
ui-settled-title = Settled decisions ({ $count })
ui-settled-help = Rules, recipes, the arbiter and you decided these. Change any that is wrong.
ui-col-decision = Decision
ui-col-folder = Folder
ui-col-size = Size
ui-col-why = Why
ui-col-change = Change
ui-change-for = Change the decision for { $path }
ui-arbiter-ask = Ask the arbiter
ui-arbiter-help = An arbiter is configured: { $url }. Nothing is sent before you see it.
ui-arbiter-preview-title = Send { $count } folder summaries?
ui-arbiter-preview-help = This is the exact request for the first folder, to { $url } (model { $model }). The others differ only by their summary. Names: { $privacy }.
ui-privacy-full = sent as they are
ui-privacy-redacted = masked under personal folders
ui-arbiter-send = Send
ui-cancel = Cancel
ui-arbiter-progress = The arbiter answered { $done } of { $total }
ui-arbiter-failed = The arbiter stopped: { $error }
ui-export-title = Export the plan and the notebook
ui-export-help = Choose a folder outside the analysed profile, for instance on a USB drive. Bellcart writes the plan (plan.json), the migration notebook (notebook.html and notebook.md) and your decisions.
ui-export-label = Export folder
ui-export-go = Write the files
ui-export-done = Written
ui-export-next = Open notebook.html now and go through "Before you leave" while this PC is still here. Then take the folder to the new PC.
ui-back = Back
ui-quit = Close Bellcart
ui-closed = Bellcart is closed. You can close this tab.
ui-root = (the whole profile)
