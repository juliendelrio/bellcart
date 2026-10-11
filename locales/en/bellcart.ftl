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
notebook-export-winget = Save the list of applications installed with winget: { $command }
notebook-export-vscode = Save the list of Visual Studio Code extensions: { $command }
notebook-export-registry = Save the registry key { $key }: { $command }
notebook-install = Install it with winget: { $command }
