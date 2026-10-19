# Sweep

A clean-room Windows uninstaller in Rust, Tauri v2 and Svelte, with features
in the spirit of Revo Uninstaller Pro. It contains no code or reverse
engineering from Revo.

**Current build: Stage A.** Sweep's own cleanup is disabled. You can:

1. list installed programs;
2. run a program's own (vendor) uninstaller;
3. check what the uninstaller actually did;
4. run a **read-only** leftover scan;
5. review the evidence for each leftover it finds.

The vendor uninstaller *does* run and changes your system. Sweep deletes
nothing.

`docs/SPEC.md` is the authoritative specification. The Stage B designs (not
implemented) are in `docs/helper-security.md` and `docs/journal.md`.

## Layout
| Path | Contents |
|---|---|
| `crates/sweep-core` | Platform-neutral logic: Windows path model, inventory, uninstall-command parser, launch plan, outcome state machine, protection rules, evidence engine, read-only scanner, Windows-semantics fakes. |
| `crates/sweep-win` | Win32 backends: registry views, filesystem, Known Folders, manifest probe, ShellExecuteEx launch and process tracking, MSI state, `PendingFileRenameOperations`. |
| `crates/sweep-fixtures` | `sweep-fixture.exe`, a controlled fake app (per-user install/uninstall) for the Windows tests. |
| `src-tauri` | Unelevated Tauri app (`asInvoker`) with commands that take only opaque IDs, a locked-down capability file and a strict CSP. |
| `ui` | Svelte 5 UI, plus Playwright smoke tests against a mocked backend. |
| `.github/workflows/sweep-windows.yml (repo root)` | Routine CI (planned). |
| `tests/vm` | Disposable-VM test plan (planned). |

## Build

On Windows (the supported platform). You need Rust stable, Node 22 and the
WebView2 runtime (preinstalled on Windows 10/11):
```powershell
cd ui; npm ci; npm run build; cd ..
npm install --global @tauri-apps/cli@2
cd src-tauri; tauri build          # NSIS installer in target\release\bundle\nsis
# or, for development:  tauri dev
```

From Linux, as a cross-check only (needs `mingw-w64` and
`rustup target add x86_64-pc-windows-gnu`):
```bash
cargo test -p sweep-core
(cd ui && npm ci && npm run build && npx playwright test)
cargo build --workspace --release --target x86_64-pc-windows-gnu
```
The GNU cross-build puts `WebView2Loader.dll` next to `sweep.exe`; ship both.

## Windows validation steps (to be run by you; not yet done)
1. Push the repo to GitHub and let `.github/workflows/sweep-windows.yml (repo root)` run. It runs:
   - the core tests on Windows;
   - `stage_a_windows` with the fixture app: inventory, success, failure and
     unknown outcomes, spawn-and-exit tracking, shared publisher, junction;
   - the UI tests;
   - the installer build.
2. Install the NSIS artifact in a **disposable VM** and work through
   `tests/vm/README.md`: UAC with a standard user, the same user and a
   different admin account, real MSI, NSIS and Inno uninstallers, reboot codes.
   Record each run in `docs/evidence/`.
3. Stage B (quarantine and restore) stays disabled until steps 1 and 2 pass
   **and** the Stage B suites in `docs/journal.md` pass.

## Test status

| Suite | Environment | Status |
|---|---|---|
| `sweep-core` unit and acceptance tests (43) | Linux, fakes with Windows semantics | **Run: 43 passed** |
| Workspace type-check and release build for `x86_64-pc-windows-gnu` | Linux + mingw | **Run: builds; `sweep.exe` embeds an `asInvoker` manifest** |
| UI type-check (`svelte-check`) and build | Linux | **Run: 0 errors** |
| UI smoke tests (2) | Linux headless Chromium, **mocked** backend | **Run: 2 passed** |
| `stage_a_windows` (6) | **Wine** on Linux (not Windows) | **Run: 5 passed, 1 failed.** The junction test fails because Wine's `mklink /J` can't create the junction. Not Windows evidence. |
| `stage_a_windows` (6) | Windows CI (`windows-latest`) | **Planned**, not run (no remote) |
| VM suite (`tests/vm`) | Disposable Windows VMs | **Planned**, not run |
| Windows MSVC build | — | Not run: the MSVC SDK download is blocked in the dev container. CI uses MSVC. |

## What is mocked, stubbed or unsupported in Stage A
- **Mocked:**
  - The UI smoke tests replace `window.__TAURI_INTERNALS__`.
  - The core acceptance tests use `FakeFs` and `FakeReg`.
- **Stubbed:** non-Windows builds of the app use an inert platform that lists
  nothing and refuses to uninstall.
- **Not implemented:**
  - Store/MSIX apps are not listed.
  - No restore points (shown as "Unavailable in this build").
  - No elevated helper.
  - No quarantine, cleanup or restore.
  - No forced uninstall, install monitor or cleaners.
- **Unsupported, and reported to you rather than run:**
  - uninstall commands that are ambiguous and unquoted with spaces, unless an
    independently validated folder disambiguates them;
  - relative or bare program names other than `rundll32`;
  - shells and scripts;
  - unresolved `%VARS%`;
  - UNC or network paths;
  - per-user uninstallers while Sweep is elevated;
  - uninstallers that need a different account.
- **Known limitations:**
  - Registry values containing 8.3 short names (`PROGRA~1`) are not expanded,
    so they don't count as evidence. This is safe but can miss ownership.
  - On-disk names with trailing dots or spaces can't be examined (Win32
    normalisation). They show up as missing, not as a coverage gap.
  - Registry symbolic links are followed when read.
  - Descendant tracking can miss children of processes Sweep can't open. This
    is reported as "tracking incomplete".
  - Matching is heuristic, so leftovers can be missed. Missing a leftover is
    the safe failure mode.

## Licence
MIT
