# Security policy

Bellcart reads the directory structure of personal machines and may run
with administrator rights. Reports are taken seriously.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's private vulnerability
reporting on this repository (Security tab, "Report a vulnerability").

Include what you observed, how to reproduce it, and the version or commit.
You will get an answer as soon as the maintainer can look at it; this is a
volunteer project with no guaranteed response time.

## What counts

- Anything that makes Bellcart write to, move or delete data on the scanned
  machine.
- Anything that sends data off the machine without the user having
  configured it, or sends more than the preview showed.
- A pack file that can cause code or a command of its choosing to run.
- Any way to turn the elevated mode into a privilege escalation.

## Supported versions

Until the first stable release, only the latest commit on `main` is
supported.
