# Elevated helper security model (Stage B design — NOT IMPLEMENTED)

Stage A has no helper and no privileged code. This document tracks the
design and its open issues so Stage B starts from the corrected model.

## Processes
- `sweep.exe`: always `asInvoker`, Medium integrity. Owns the UI.
- `sweep-helper.exe`: `requireAdministrator`, High integrity, started on demand
  with `ShellExecuteExW("runas", SEE_MASK_NOCLOSEPROCESS)`.

## Trusted location
Both binaries must live in `%ProgramFiles%\Sweep` (NSIS per-machine install),
a directory whose owner is Administrators/TrustedInstaller and which grants
Users no write access. A filename alone proves nothing. At startup each side
checks the *peer's* image path (`QueryFullProcessImageNameW`) is inside that
directory and that the directory's DACL still denies Users write. Dev builds
outside that location refuse to start the helper unless built with the
`dev-insecure` feature, which the UI announces.

## Pipe
- Name: `\\.\pipe\sweep-<128-bit random>` created by the app (server) with
  `FILE_FLAG_FIRST_PIPE_INSTANCE`, `PIPE_REJECT_REMOTE_CLIENTS`, one instance.
- DACL (principals, not processes):
  `D:P(A;;GRGW;;;<interactive user SID>)(A;;GRGW;;;S-1-5-32-544)`.
  The elevated helper's token has BUILTIN\Administrators enabled — including
  when elevation used a *different* admin account. Any elevated process could
  pass the DACL, so the DACL is not authentication.
- Integrity label: none set. A High-IL client opening a Medium-IL object is
  allowed by the default no-write-up policy; the policy restricts lower-IL
  callers only. **To be verified on Windows** for all three elevation cases.

## Peer authentication (separate from the DACL)
- App → helper: `GetNamedPipeClientProcessId` must equal `GetProcessId` of the
  handle returned by `ShellExecuteExW`. The app holds that handle, so the PID
  cannot be reused while connected. Image path must be the trusted helper.
- Helper → app: `GetNamedPipeServerProcessId` must equal `--parent-pid`; the
  helper opens that process immediately and verifies its image path, then
  reads the **user SID from the server's token** (`OpenProcessToken`). That
  SID — not anything in a message — defines whose `HKU\<SID>` and Known
  Folders (via the duplicated token) the helper operates on.

## Session protocol
- First frame `Hello { version, session_id (128-bit random) }`; exact version
  match; one session, one connection; helper exits on disconnect or 5 min idle.
- Every request carries `session_id` and a strictly increasing `seq`; stale or
  duplicate `seq`, foreign session, unknown fields (`deny_unknown_fields`),
  or frames > 1 MiB close the connection. 30 s per-frame read timeout.
- Closed request enum: `TakeSnapshot`, `CreateRestorePoint`,
  `ExecuteCleanupPlan`, `RestoreItems`, `RecoverInterrupted`, `Shutdown`.
  No "run command", no "delete path".

## Trustworthy pre-uninstall snapshot
Hashing data supplied by the unelevated app does not authenticate it. In
Stage B the helper is started **before** the vendor uninstaller runs and takes
the evidence snapshot itself (HKLM, `HKU\<SID>`, MSI component registrations,
Known Folders via the user token), stores it in an Administrators-only
`%ProgramData%\Sweep\snapshots\<id>`, and returns only the ID. Cleanup plans
reference that ID; the helper never accepts snapshot contents from the app.

## Authorization of a cleanup
A plan is `{op_id, snapshot_id, plan_version, candidate_ids}`; the user's
confirmation binds to `sha256(plan)`. The helper re-derives every candidate
from its own snapshot plus live reads, re-runs protection and evidence
rules, rejects `Blocked`, accepts `Review` items only if listed in this exact
plan, and revalidates identity immediately before each mutation.

## Open issues to resolve before implementation
1. Confirm DACL + no-label behaviour with standard-user, same-user and
   other-admin elevation on Windows 10 and 11 (VM tests).
2. Decide whether `S-1-5-32-544` is acceptable or a per-session logon SID can
   be used instead.
3. Restore-point creation: verify `SystemRestore.CreateRestorePoint` success
   by sequence number (the API can report success without creating a point).
