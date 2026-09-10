# Security

## Reporting

Report a vulnerability privately through GitHub's "Report a vulnerability" form on
this repository's Security tab. Expect an acknowledgement within a week.

## What the app does that matters

layoutswap writes PowerShell scripts from the user's own configuration and runs
them with the user's own privileges. Three properties limit what that can do:

- Script text is rendered in Rust from typed values. The window never sends script
  text to the backend (ADR-0003), so a compromised renderer cannot run arbitrary
  commands.
- Scripts are written under the user's profile and run without elevation. The one
  action that needs an elevated prompt, registering a session-unlock trigger, is
  requested explicitly and can be declined.
- Generated scripts and their logs are plain text the user can read before and
  after a run.

Remote Desktop connection files contain hostnames. The app edits only the
`selectedmonitors` line and never copies, uploads or logs the rest of the file.
