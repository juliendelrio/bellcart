# English is the reference language: every key must exist here.

decision-migrate = Migrate
decision-archive = Archive
decision-leave = Leave behind
decision-review = To review

plan-title = Plan for { $root }
plan-settled = Settled by rules and recipes
plan-settled-empty = { $count ->
    [one] and 1 empty folder, not listed
   *[other] and { $count } empty folders, not listed
}
plan-nothing-settled = No rule matched anything here.
plan-unreadable = { $count ->
    [one] { $count } folder could not be read.
   *[other] { $count } folders could not be read.
} If Bellcart did not run as administrator, doing so includes them.
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
reason-synced-synology-drive = Folder synced with a Synology NAS: its content is also on the NAS, except files waiting to upload and files the sync rules leave out, often every name starting with a dot (.env, .git, .claude)
reason-other-applications = Applications too small for a card of their own (under 100 MB). Their names are listed below
reason-tool-folder = Settings and data of a tool or an editor, kept in a dot folder: look at what it holds, or archive it when in doubt
reason-app-extensions = Extensions: those from the marketplace are installed again from the list the recipe saves
reason-unknown-application = Folder of an application Bellcart does not know: look at what it holds, or archive it when in doubt
reason-app-cloud-account = Its data is on the servers of the service: the application gets it back when you sign in again
reason-app-sync-client = A sync client and its state: it rebuilds them when you connect it again
reason-app-downloaded-tools = Tools downloaded automatically, downloaded again when needed
reason-app-browser-profile = A browser profile: the recipe or your account's sync bring back bookmarks and settings. The rest is mostly site data: archiving keeps it without copying it
reason-app-editor-state = The editor's state (local history, workspaces): the recipe already takes the settings. Archiving keeps the rest without copying it
reason-app-mail = Mail and address books stored on this PC: they may exist only here
reason-app-settings = The application's settings and data, usually small: migrate them to find it as you left it
reason-app-virtual-disks = Disks of virtual machines or containers: they may hold your data (volumes, Linux distributions). Archiving keeps them without copying them to the new PC
reason-crash-reports = Crash reports, only of use to the application's developers
reason-installed-program = Installed program, to install again on the new PC
reason-update-download = Update already downloaded, fetched again when needed
reason-public-profile = Folders every account of the PC can see: files may have been put there to share them
reason-other-profile = Another account of this PC: its documents belong to someone else, ask them before deciding
reason-bellcart-export = Bellcart's plan, notebook and decisions: you need them on the new PC
reason-previous-windows = Left by an earlier Windows installation, with the profiles it had: it may hold the only copy of old files
reason-installer-cache = Installers kept so that programs can repair or remove themselves: the new PC gets its own
reason-git-repository = A git repository: what is pushed is also on its server, but not changes not yet committed or pushed, nor the files git ignores (.env and the like). Migrating it keeps everything; the notebook says how to check
reason-code-project = A code project without git: it may exist nowhere else. Its dependencies and build output are left already
reason-claude-worktrees = Working copies Claude Code made of this repository: their commits are in it, but changes not committed exist only here

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
notebook-programs-undecided = Programs to decide about
notebook-program-decide = Decide whether you need { $name } on the new PC, in the check step of bellcart ui
notebook-programs-reinstall = Programs to install again
notebook-program-install = Install { $name } again
notebook-repositories = Git repositories
notebook-repositories-run = In each repository below, run this in PowerShell: { $command }. What it lists exists only on this PC: commit and push it, or migrate the folder
notebook-repository = Nothing left to push in { $path }
notebook-repositories-ignored = Files git ignores, such as .env, settings.local.json or CLAUDE.local.md, are on no server: to clone a repository again instead of copying it, copy those files first

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
ui-listening-no-browser = Bellcart is ready. Open this address in your browser: { $url } (Ctrl+C here to stop it)
ui-not-a-folder = { $path } is not a folder.
ui-title = Bellcart
ui-step-choose = Choose
ui-step-analyse = Analyse
ui-step-review = Review
ui-step-summary = Check
ui-step-export = Export
ui-choose-title = Which profile do you want to sort?
ui-choose-help = Bellcart only reads. It never moves, changes or deletes anything on this PC, and nothing leaves it unless you ask.
ui-choose-label = Folder to analyse
ui-choose-start = Analyse
ui-resume-title = Pick up where you left off?
ui-resume-help-one = { $count } decision is saved for { $profile }, in { $file }.
ui-resume-help-other = { $count } decisions are saved for { $profile }, in { $file }.
ui-resume = Resume
ui-start-over = Start over (the saved decisions will be replaced)
ui-resumed-one = { $count } decision resumed
ui-resumed-other = { $count } decisions resumed
ui-saved-to = Your decisions are saved as you make them, in { $file }.
ui-not-saved = Your decisions are not saved: the proposed export folder is inside the analysed folder.
ui-save-failed = The decision is taken but could not be saved: { $error }
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
ui-card-propose = Proposed: { $decision }.
ui-key-enter = Enter
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
ui-go-export = Check and export
ui-summary-title = Check before exporting
ui-summary-help = This is what the plan and the notebook will say. Nothing is written until you export, and any decision can still be changed.
ui-summary-split = How the profile is split
ui-summary-share = { $decision }: { $size }, { $percent } %
ui-summary-attention = Points to check
ui-summary-all-clear = Nothing needs a second look: every folder is decided, and nothing was left behind by a choice of yours.
ui-summary-undecided-one = { $count } folder not decided ({ $size }). It stays "to review" in the plan: nothing will be done for it.
ui-summary-undecided-other = { $count } folders not decided ({ $size }). They stay "to review" in the plan: nothing will be done for them.
ui-summary-left-by-you-one = { $count } folder you chose to leave behind ({ $size }). Make sure nothing in it exists only on this PC.
ui-summary-left-by-you-other = { $count } folders you chose to leave behind ({ $size }). Make sure nothing in them exists only on this PC.
ui-summary-left-personal-one = { $count } folder in your personal folders is left by a rule ({ $size }). Check that the reason fits.
ui-summary-left-personal-other = { $count } folders in your personal folders are left by a rule ({ $size }). Check that the reason fits.
ui-summary-unreadable-one = { $count } folder could not be read, so it is not in the plan. If Bellcart did not run as administrator, doing so includes it.
ui-summary-unreadable-other = { $count } folders could not be read, so they are not in the plan. If Bellcart did not run as administrator, doing so includes them.
ui-summary-largest = Largest folders in each cart
ui-summary-cart-empty = Nothing in this cart.
ui-summary-decide = Decide
ui-summary-change = Change
ui-summary-applications = The notebook will say what to do, before leaving and on arrival, for: { $names }
ui-summary-programs = Programs to install again
ui-summary-programs-help = These programs are installed on this PC. Their files are not copied: say which ones you want on the new PC, and the notebook will list them to install again.
ui-summary-programs-undecided-one = { $count } program is not decided yet. The notebook will ask you to decide before leaving.
ui-summary-programs-undecided-other = { $count } programs are not decided yet. The notebook will ask you to decide before leaving.
ui-reinstall-for = Install { $name } again on the new PC?
ui-reinstall-yes = Install again
ui-reinstall-no = Not needed
ui-reinstall-undecided = Not decided
ui-summary-confirm = Looks right, go to export
ui-summary-back = Back to the review
ui-settled-title = Settled decisions ({ $count })
ui-settled-help = Rules, recipes, the arbiter and you decided these. Change any that is wrong.
ui-settled-filter = Find a folder
ui-settled-showing = { $shown } of { $count } shown
ui-settled-more = Show { $count } more
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
ui-export-help = Choose a folder outside the analysed folder, for instance on a USB drive. Bellcart writes the plan (plan.json), the migration notebook (notebook.html and notebook.md) and your decisions.
ui-export-inside = This folder is inside the analysed folder ({ $profile }): Bellcart never writes there. Choose a folder outside it, on a USB drive for instance.
ui-export-inside-drive = This folder is on the analysed drive ({ $profile }): Bellcart only writes there in its own folder, { $own }. Choose that one, or better a folder on a USB drive.
ui-export-local = The proposed folder is on the analysed drive: it is the only folder Bellcart creates there. A USB drive or an external disk is better: you will need these files on the new PC. Avoid a drive synchronised with the cloud (Google Drive, OneDrive): your plan would be uploaded.
ui-export-label = Export folder
ui-export-go = Write the files
ui-export-done = Written
ui-export-next = Open notebook.html now and go through "Before you leave" while this PC is still here. Then take the folder to the new PC.
ui-back = Back
ui-quit = Close Bellcart
ui-closed = Bellcart is closed. You can close this tab.
ui-root = (the whole profile)
ui-crumb-up = Decide from { $path }, with all it contains
ui-scope-title = Decide for all of { $path }
ui-scope-pending-one = { $count } folder still to decide below it ({ $size })
ui-scope-pending-other = { $count } folders still to decide below it ({ $size })
ui-scope-changed-one = { $count } decision already made below it will be replaced
ui-scope-changed-other = { $count } decisions already made below it will be replaced
ui-scope-kept-one = { $count } folder settled by a rule or a recipe keeps its decision
ui-scope-kept-other = { $count } folders settled by a rule or a recipe keep their decision
ui-scope-back = Back to { $path } only
ui-decided-below-one = { $path } and its content: { $decision } ({ $count } change)
ui-decided-below-other = { $path } and its content: { $decision } ({ $count } changes)
