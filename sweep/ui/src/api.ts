// Typed wrapper over the backend commands. The UI only ever sends opaque IDs.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface EnvStatus {
  stage: string;
  cleanupEnabled: boolean;
  restorePoint: string;
  elevated: boolean;
  supportedPlatform: boolean;
  inventoryUnreadable: string[];
}

export interface Program {
  id: string;
  name: string;
  publisher: string | null;
  version: string | null;
  scope: string;
  views: string[];
  msi: boolean;
  sizeKb: number | null;
  installDate: string | null;
  hasUninstaller: boolean;
}

export interface Plan {
  verb: 'open' | 'runas';
  file: string;
  params: string;
  reason: string;
  elevation: string;
}

export interface Preview {
  program: Program;
  installDir: string | null;
  installDirRejected: string | null;
  commandSummary: string | null;
  commandError: string | null;
  plan: Plan | null;
  refusal: string | null;
  restorePoint: string;
  restorePointNote: string;
  canLaunch: boolean;
}

export interface OutcomeView {
  kind: 'succeeded' | 'cancelled' | 'failed' | 'unknown';
  label: string;
  detail: string;
  autoScanAllowed: boolean;
}

export interface OpView {
  opId: string;
  programName: string;
  state: 'ready' | 'launching' | 'running' | 'verifying' | 'finished';
  pids: number[];
  trackingIncomplete: boolean;
  outcome: OutcomeView | null;
}

export interface CandidateView {
  id: string;
  target: string;
  kind: 'folder' | 'file' | 'registry';
  verdict: 'likely' | 'review' | 'blocked';
  discoveredBy: string[];
  ownership: string[];
  shared: string[];
  userData: string[];
  concerns: string[];
  protection: string | null;
  stats: { files: number; dirs: number; bytes: number } | null;
}

export interface CoverageView {
  location: string;
  status: string;
  detail: string | null;
  gap: boolean;
}

export interface ReportView {
  candidates: CandidateView[];
  coverage: CoverageView[];
  complete: boolean;
  cancelled: boolean;
  uncertain: boolean;
  cleanupEnabled: boolean;
  itemsVisited: number;
}

export interface ScanView {
  scanId: string;
  phase: string;
  itemsVisited: number;
  candidateCount: number;
  done: boolean;
  report: ReportView | null;
}

export const api = {
  envStatus: () => invoke<EnvStatus>('env_status'),
  listPrograms: () => invoke<Program[]>('list_programs'),
  previewUninstall: (programId: string) => invoke<Preview>('preview_uninstall', { programId }),
  startUninstall: (programId: string) => invoke<string>('start_uninstall', { programId }),
  stopWaiting: (opId: string) => invoke<void>('stop_waiting', { opId }),
  getOperation: (opId: string) => invoke<OpView>('get_operation', { opId }),
  startScan: (src: { opId?: string; programId?: string }, acknowledgeUncertain: boolean) =>
    invoke<string>('start_scan', { opId: src.opId ?? null, programId: src.programId ?? null, acknowledgeUncertain }),
  cancelScan: (scanId: string) => invoke<void>('cancel_scan', { scanId }),
  getScan: (scanId: string) => invoke<ScanView>('get_scan', { scanId }),
};

export function onUninstallUpdate(cb: (v: OpView) => void): Promise<UnlistenFn> {
  return listen<OpView>('uninstall-update', (e) => cb(e.payload));
}
export function onScanProgress(cb: (v: ScanView) => void): Promise<UnlistenFn> {
  return listen<ScanView>('scan-progress', (e) => cb(e.payload));
}
export function onScanDone(cb: (v: ScanView) => void): Promise<UnlistenFn> {
  return listen<ScanView>('scan-done', (e) => cb(e.payload));
}

export function formatSize(kb: number | null): string {
  if (kb == null) return '';
  if (kb >= 1024 * 1024) return `${(kb / 1024 / 1024).toFixed(1)} GB`;
  if (kb >= 1024) return `${(kb / 1024).toFixed(1)} MB`;
  return `${kb} KB`;
}

export function formatBytes(b: number): string {
  return formatSize(Math.round(b / 1024));
}

export function formatDate(yyyymmdd: string | null): string {
  if (!yyyymmdd || !/^\d{8}$/.test(yyyymmdd)) return yyyymmdd ?? '';
  return `${yyyymmdd.slice(0, 4)}-${yyyymmdd.slice(4, 6)}-${yyyymmdd.slice(6, 8)}`;
}
