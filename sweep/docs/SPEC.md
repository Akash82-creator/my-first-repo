# Sweep specification (authoritative)

A clean-room Windows uninstaller in Rust, Tauri v2 and Svelte. Revo Uninstaller Pro is not
reverse engineered. The repo is `/home/user/sweep`, a new git repo with no remote. This
spec replaces every earlier draft, and it is copied into the repo as `docs/SPEC.md`.

## Context
The user wants an app with Revo-like features, with deletion safety, recovery and honest
verification treated as the hard part. Delivery is staged. **This session implements
Stage A only:**
- a real Windows inventory;
- running the selected vendor uninstaller;
- checking the outcome;
- a real, read-only leftover scan;
- evidence review.

Sweep's own cleanup is **disabled**. Vendor uninstallers do run and do change the system.

## Invariants
1. A name or publisher match only finds a candidate. It never preselects one.
2. The UI process never runs elevated. If Sweep finds that it is elevated, it refuses to
   launch per-user uninstallers and shows a warning banner.
3. The frontend only sends opaque IDs (program, operation, scan). It never sends paths or
   command lines to run, and the backend resolves every ID from sessions it holds itself.
4. Sweep only promises to undo its own cleanup (Stage B). It never claims it can undo the
   vendor's uninstaller.
5. A scan with skipped coverage is never shown as "no leftovers found".

## Workspace
```
crates/sweep-core      platform-neutral, tested on Linux: WinPath, inventory normalization,
                       uninstall-command parser, MSI codes, uninstall state machine,
                       protection rules, evidence engine, read-only scanner with coverage,
                       progress and cancellation, fakes (feature "fakes")
crates/sweep-win       #[cfg(windows)] backends: registry with explicit WOW64 views,
                       read-only FsApi, Known Folders, manifest elevation probe, launching
                       through ShellExecuteExW, process and descendant tracking,
                       MsiQueryProductStateW, PendingFileRenameOperations, token elevation
crates/sweep-fixtures  fixture installer and uninstaller binaries for Windows tests
src-tauri              unelevated app, opaque-ID commands, sessions, events
ui/                    Svelte + TS (Programs, Uninstall, Review views)
docs/                  SPEC.md, helper-security.md (Stage B), journal.md (Stage B)
.github/workflows/     windows.yml (planned CI)
tests/vm/              disposable-VM scripts (planned; they don't count as passing)
```

## Stage A behaviour

### Inventory
- Reads `HKLM\...\Uninstall` in both the 64-bit and 32-bit views and `HKCU\...\Uninstall`
  (HKCU has no separate view on modern Windows).
- Hides `SystemComponent=1`, entries with `ParentKeyName`, `ReleaseType` values for
  updates and hotfixes, and entries with no `DisplayName` and no uninstall command.
- **Deduplication:** by `(scope, key name)` first, then by ProductCode. A duplicate shows
  both registry views as its sources.
- Each program records its scope (Machine or User), registry view and key path.

### Uninstall command parsing (`cmdline.rs`)
- `MsiExec.exe /X{GUID}` and `/I{GUID}` (with or without a path, any case) are parsed as
  `Msi{product_code}`. `WindowsInstaller=1` plus a GUID key name also counts as MSI.
- **Quoted executable:** the text inside the quotes is the exe, and the rest is kept as
  the exact argument tail with leading whitespace removed.
- **Unquoted, no spaces in the first token:** if the first token ends in `.exe`, that is
  the exe.
- **Unquoted with spaces:** candidates are the space-boundary prefixes ending in `.exe`.
  Exactly one candidate is accepted, and only if it exists and its parent folder equals the
  **validated** InstallLocation or the DisplayIcon folder. Otherwise the command is
  `Unsupported(Ambiguous)`.
  - `C:\Program.exe` is never considered, because the prefix `C:\Program` doesn't end
    in `.exe`.
- **Bare names:** `rundll32` and `rundll32.exe` map to `%SystemRoot%\System32\rundll32.exe`.
  Any other bare or relative name is `Unsupported(Relative)`.
- `%VAR%` is expanded from the environment, and an unresolved variable is
  `Unsupported(UnresolvedVariable)`.
- `cmd`, PowerShell, `.bat`, `.cmd`, `.ps1`, `.vbs` and `.js` are
  `Unsupported(ScriptOrShell)`, and so is any target that isn't an existing `.exe`.
- Every unsupported form is shown to the user and never run.

### Elevation and launching (UI stays unelevated)
- **MSI:** runs `%SystemRoot%\System32\msiexec.exe /x {GUID}` through `ShellExecuteExW`
  with the verb `open`. Windows Installer handles elevation for per-machine products.
- **EXE, per-user (HKCU):** verb `open`. If Sweep is elevated this is refused, because it
  would run under the wrong user.
- **EXE, per-machine (HKLM):** Sweep reads the exe's embedded `RT_MANIFEST`
  `requestedExecutionLevel`.
  - `requireAdministrator` or `highestAvailable` means verb `open`; the manifest elevates.
  - `asInvoker`, a missing manifest or an unreadable one means verb `runas`, so Windows
    shows UAC for **that process only**.
  - The plan shown before launching tells the user which path will be taken.
- **UAC cancelled:** ShellExecuteEx fails with `ERROR_CANCELLED` (1223), which becomes
  outcome `ElevationCancelled`.
- **Other launch failures:** `ElevationFailed(code)` or `LaunchFailed(code)`.
- **Not supported:** uninstallers that need a different user account than the one running
  Sweep (we report it), and running Sweep elevated for per-user apps.
- **Restore points:** shown before uninstalling as **"Unavailable in this build"**, deferred
  to Stage B because it needs the privileged helper. The user can still continue.

### Outcome state machine
```
Ready → Launching → Running → Verifying →
  Succeeded | SucceededRebootRequired | RebootInitiated | Cancelled |
  ElevationCancelled | Failed(code) | LaunchFailed(code) | Unknown(reason)
```
- **Tracking:** the process handle (`SEE_MASK_NOCLOSEPROCESS`), plus descendants found by
  polling Toolhelp snapshots for parent-PID chains with later creation times. If a
  descendant can't be opened, tracking is marked `incomplete`.
- **MSI exit codes:**
  - 0: Succeeded, confirmed by `MsiQueryProductStateW == INSTALLSTATE_UNKNOWN`; otherwise
    Unknown;
  - 1602: Cancelled;
  - 1605: Unknown(NotInstalled);
  - 1618: Failed (retryable);
  - 1641: RebootInitiated;
  - 3010: SucceededRebootRequired;
  - anything else: Failed.
- **EXE outcomes:**
  - exit 0 and the Uninstall entry is gone: Succeeded;
  - exit 0 and the entry is still there: Unknown(EntryStillPresent);
  - non-zero: Failed(code);
  - new `PendingFileRenameOperations` entries under the install folder: the reboot-required
    variant.
- **Leftover scan:** runs automatically only after Succeeded, SucceededRebootRequired or
  RebootInitiated. After any other outcome the user can only start a scan explicitly, and
  it is labelled "outcome uncertain". Nothing is ever cleaned (Stage A).

### Pre-uninstall evidence snapshot
Taken in the unelevated app before launching:
- the full Uninstall entry;
- the validated InstallLocation;
- the folders of DisplayIcon and the uninstaller;
- the other programs' install folders;
- tokens built from the product and publisher names;
- the install date.

The leftover scan uses this snapshot, because the entry may be gone afterwards. (Stage B
trust model: see below.)

### Protection rules
- **Protected containers** can't be targets themselves, but their descendants are still
  evaluated. They are: volume roots, Program Files (both), Program Files\Common Files,
  ProgramData, Users, each profile root, every Known Folder root (AppData, LocalAppData,
  Documents, Desktop, Start Menu…), and Windows\Temp. They are resolved through Known
  Folders, never hard-coded.
- **Protected subtrees** block a path and everything under it: Windows, System32 and
  SysWOW64, WinSxS, Sweep's own folders, and other installed programs' validated
  InstallLocations.
- **InstallLocation validation:** it must be absolute, at least one level below a
  protected container, not a container, not inside a protected subtree, and not equal to
  another program's location.
- **External path syntax** (strings from the registry) is checked strictly: UNC, device
  paths, ADS, drive-relative and rooted-relative paths, and `..` are rejected. Paths
  returned by Windows APIs go through a separate normalizer that accepts `\\?\` final paths.
- **Comparison:** ordinal case-insensitive, with trailing separators, dots and spaces
  trimmed. Core never uses `std::path` for Windows paths; tests only use Windows-style
  strings.

### Leftover scan (read-only)
- **Locations:**
  - the validated install folder;
  - folders named after the product or publisher under Program Files (x86 and x64),
    ProgramData, AppData, LocalAppData and LocalAppData\Programs;
  - Start Menu Programs (user and common) plus Desktop `.lnk` names;
  - `HKCU` and `HKLM` (both views) `Software\<Publisher>` and `Software\<Product>`;
  - the leftover Uninstall entry;
  - `App Paths` entries that point inside the install folder.
- **Coverage:** every place visited records a status: `Scanned`, `Skipped(AccessDenied |
  NotFound | ReparsePointNotFollowed | DepthLimit | Error)` or `Cancelled`. The report says
  whether coverage was **complete** or **incomplete**, and a gap is never reported as
  "nothing found".
- **Reparse points** are never followed. They are reported as coverage gaps and push the
  candidate down to Review.
- **Progress and cancellation:** a progress event goes out every 100 items or 200 ms.
  Cancelling uses an atomic token and produces a partial report marked "cancelled".
- **Candidate fields:** `target`, `discovered_by[]` (why it matched), `ownership[]`
  (strong evidence), `shared_signals[]`, `user_data[]`, `protection`, `verdict`.
- **Verdicts:**
  - `Blocked`: protected.
  - `Likely`: strong ownership evidence, no shared signals, no user data. In Stage B this
    would be preselected; Stage A only shows it.
  - `Review`: everything else.
  Name-only matches are always Review or below. A shared publisher folder is never Likely,
  and only its product subfolder is evaluated.
- **User data:** files under Documents, Desktop, Pictures and similar folders; files
  modified after the install date; document or database extensions.

### UI
- **Programs:** search and sort, with scope, view and MSI badges.
- **Uninstall:** shows the parsed command, the elevation path and "Restore point:
  unavailable in this build" before you confirm. After launching it shows the live state
  and the outcome.
- **Review:** shows candidates grouped by verdict, each with its reasons and evidence, plus
  a coverage panel that is prominent when coverage is incomplete. There are no delete
  controls; a banner says "Cleanup is disabled in this build."
- **Tauri:** a capability file allows only Sweep's commands, with no shell, fs or opener
  plugins. The strict CSP is
  `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:;
  connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none';
  frame-ancestors 'none'`. `freezePrototype` is on, and the manifest is `asInvoker`.

## Stage B (disabled; tracked issues, not built in this session)
- **Elevated helper, IPC:** the app is the pipe server with `FIRST_PIPE_INSTANCE` and
  `REJECT_REMOTE_CLIENTS`.
  - **DACL:** grants exactly two SIDs, the interactive user's SID and BUILTIN\Administrators
    (S-1-5-32-544, the group the elevated helper's token has enabled). A DACL can't name a
    process, so the helper is authenticated **separately**:
    - the client PID must equal the PID of the process handle from ShellExecuteEx;
    - the image path must be inside the trusted install folder
      (`%ProgramFiles%\Sweep`, owned by Administrators or TrustedInstaller, not writable by
      Users);
    - in the other direction, the helper checks the server's PID and image path.
  - **Integrity level:** no explicit label. The Medium-integrity pipe can be opened by the
    High-integrity client.
  - **Session hygiene:** a session-ID handshake, a strictly increasing `seq` (anti-replay),
    1 MiB frames, 30 s frame timeout, 5 min idle exit, `deny_unknown_fields`.
  - **Testing:** standard user, same-user elevation and other-admin elevation are tested
    in VMs.
- **Trustworthy snapshot:** the helper starts **before** the vendor uninstaller runs and
  takes the pre-uninstall evidence snapshot **itself** from HKLM, `HKU\<SID>` and the MSI
  APIs. It stores the snapshot in an admin-only ProgramData folder and returns only an ID,
  so data from the unelevated app is never trusted. Restore-point creation also lives in
  the helper and checks the sequence number.
- **Authorization:** a plan is bound to the op ID, the plan version, the plan's SHA-256 and
  the exact candidate IDs. The helper re-runs the protection and evidence checks against
  its own snapshot and live reads.
- **Quarantine:**
  - (a) Same-volume handle renames, with a durable intent holding the tree's identities.
    On recovery, each object is located by FileId.
  - (b) Copy-verify-remove across volumes and for registry keys.
  Records have a CRC32C. Recovery has separate restore states, reconciles each object
  (source only, destination only, both, neither, partial trees, partial registry), and
  never discards the only valid backup.
- **Directory races:** every child is checked against the manifest. Inserted children stay
  where they are, and modified or replaced children or a re-pointed ancestor cause a skip.
  Concurrent-mutation tests cover this.
- **Gate:** Stage B is enabled only once the Windows integration tests **and** the recorded
  VM runs pass.
- **Later milestones:** forced uninstall (review-only when started from a name), MSIX
  removal, services, tasks and associations handlers, the install monitor (observed
  changes, drift checks), cleaners (opt-in browser categories, reversible autorun disable).

## Verification in this session
- `cargo test -p sweep-core`, which includes the Stage A acceptance tests:
  - `shared_publisher_two_products`
  - `unrelated_name_match_not_preselected`
  - `user_data_inside_installdir_review_only`
  - `junction_in_installdir_not_followed`
  - `uninstall_cancelled_blocks_auto_scan`
  - `program_exe_prefix_rejected`
  - `access_denied_reported_as_coverage_gap`
  - `scan_cancellation_partial_report`
- The Windows cross-check is `cargo xwin check` or a `windows-gnu` check, whichever
  toolchain can be fetched, and it is reported exactly as it runs.
- The UI build, plus a Playwright smoke test against a mocked `invoke`.
- **Planned, not run:** the Windows CI fixture tests (`windows.yml`) and the VM suite.
- The final report lists everything mocked, stubbed, unsupported or not run.
