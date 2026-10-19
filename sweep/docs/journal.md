# Journal, quarantine and recovery (Stage B design — NOT IMPLEMENTED)

Stage A contains no mutation code. This is the design Stage B must implement
and test before any cleanup is enabled.

## Records
`%ProgramData%\Sweep\ops\<op-id>\journal.jsonl`, Administrators+SYSTEM only.
Each line: `{seq, crc32c, record}`. A record is durable only after the append
**and** `FlushFileBuffers` return. On read, the first record with a bad CRC or
truncated line ends the trusted journal; everything after is "unknown" and
forces state inspection.

## Two quarantine protocols
### (a) Same-volume rename (files and directories)
The rename *is* the mutation; there is no prior independent copy.
1. `IntentDurable{object, tree manifest: FileId,size,mtime,reparse tag per node}`
2. Revalidate (handle opened with `FILE_FLAG_OPEN_REPARSE_POINT`, identity,
   final path, every manifest child), then rename **by handle** into
   `<volume>\$Sweep.Quarantine\<op>\`.
3. `MovedDurable{object}`.
Recovery: locate each manifest node by FileId at source and destination.
Exactly one location is expected; both or neither is a reported anomaly.
Inserted children not in the manifest are left at the source.

### (b) Copy-verify-remove (cross-volume files, registry keys)
1. `IntentDurable` → copy (BackupRead/BackupWrite; registry: typed JSON with
   security descriptor) → flush → verify hash → `BackupDurable`.
2. Remove source objects one by one (`RemovedDurable{node}` per node).
Recovery: for each node, source present + backup verified → nothing lost;
source missing + backup verified → restorable; source missing + backup
unverifiable → **anomaly, never discard anything**.

## Per-object reconciliation cases (each needs a crash test)
source-only · destination-only · both-present · neither-present ·
partial tree · partially deleted registry key · corrupted/truncated record.

## Restore
States `RestoreStarted → RestoreItemStarted → RestoreItemDone → Restored`,
each durable, so an interrupted restore is itself recoverable. Conflicts:
skip, restore beside (`name (restored YYYY-MM-DD)`), or replace (current
object quarantined first). Registry: value-level diff; add missing values
only, unless replace is chosen. Backups are never deleted while they are the
only valid copy.

## Gate
Stage B stays disabled until the Windows integration suite **and** recorded
VM runs pass for every case above.
