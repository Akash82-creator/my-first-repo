# Disposable-VM tests (Stage A)

**Status: PLANNED. Nothing in this folder has been executed.** Supplying
these scripts does not count as passing them. Record real runs in
`docs/evidence/` (date, Windows build, VM snapshot, log).

These cover what routine CI cannot: real third-party uninstallers, UAC
prompts, different accounts, and reboots. Run them only in a disposable VM
(Windows Sandbox for single-session cases, Hyper-V checkpoints for reboots).

| Case | How | Pass criteria |
|---|---|---|
| Standard user, per-machine app | Log in as non-admin; uninstall 7-Zip (MSI) | UAC asks for admin credentials for msiexec only; Sweep stays Medium IL (Process Explorer); outcome Succeeded |
| Same-user elevation | Admin user, UAC on; uninstall Notepad++ (NSIS) | UAC prompt for the uninstaller only; outcome Succeeded; leftover scan auto-starts |
| UAC declined | Decline the prompt | Outcome "Administrator approval was declined"; no auto-scan |
| Other admin account (over-the-shoulder) | Standard user, enter a different admin's credentials | Per-machine uninstall works; per-user programs are listed for the logged-in user; review shows that user's AppData |
| Spawn-and-exit uninstaller | Inno Setup app (e.g. a small open-source tool) | Outcome Succeeded (not Unknown) |
| Uninstaller that opens a browser | App whose uninstaller opens a survey page | "Stop waiting" ends tracking; outcome marked "tracking incomplete" |
| Reboot required | MSI that returns 3010 | Outcome "restart required"; scan allowed |
| Reboot initiated | MSI run that returns 1641 (Hyper-V checkpoint) | After restart, Sweep shows no stuck operation; nothing was cleaned |
| Shared publisher (real) | Two products from one vendor, remove one | Other product's folders/keys never Likely; publisher folder Blocked/Review |
| Access denied coverage | Deny the user read on a ProgramData subfolder | Coverage incomplete, location listed |
| Redirected Documents | Documents redirected to D: | Documents treated as user data on D: |

`sandbox.wsb` maps a host folder with the built `sweep.exe` and
`sweep-fixture.exe` into a Windows Sandbox session.
