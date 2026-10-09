# English is the reference language: every key must exist here.

decision-migrate = Migrate
decision-archive = Archive
decision-leave = Leave behind
decision-review = To review

plan-title = Plan for { $root }
plan-settled = Settled by rules
plan-nothing-settled = No rule matched anything here.
plan-unreadable = { $count ->
    [one] { $count } folder could not be read.
   *[other] { $count } folders could not be read.
} Run as administrator to include them.
plan-files = { $count ->
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
