// UI smoke tests against a MOCKED backend (window.__TAURI_INTERNALS__).
// These verify UI behaviour only; they say nothing about Windows behaviour.
import { expect, test, type Page } from '@playwright/test';

type Scenario = 'success' | 'cancelled';

async function mockBackend(page: Page, scenario: Scenario) {
  await page.addInitScript((scenario: Scenario) => {
    const callbacks = new Map<number, (e: unknown) => void>();
    const listeners: Record<string, number[]> = {};
    let next = 1;
    const calls: Array<{ cmd: string; args: unknown }> = [];
    (window as any).__calls = calls;
    const emit = (event: string, payload: unknown) =>
      (listeners[event] ?? []).forEach((id) => callbacks.get(id)?.({ event, id, payload }));

    const program = (id: string, name: string) => ({
      id, name, publisher: 'Acme Corporation', version: '2.3', scope: 'All users', views: ['64-bit'],
      msi: false, sizeKb: 2048, installDate: '20240101', hasUninstaller: true,
    });
    const report = {
      complete: false, cancelled: false, uncertain: scenario !== 'success', cleanupEnabled: false, itemsVisited: 42,
      coverage: [
        { location: 'Program Files: C:\\Program Files', status: 'scanned', detail: null, gap: false },
        { location: 'ProgramData: C:\\ProgramData', status: 'access denied', detail: null, gap: true },
      ],
      candidates: [
        { id: 'c0', target: 'C:\\Program Files\\Acme\\Notes', kind: 'folder', verdict: 'likely', discoveredBy: ["program's InstallLocation"],
          ownership: ["is the program's validated InstallLocation"], shared: [], userData: [], concerns: [], protection: null,
          stats: { files: 2, dirs: 1, bytes: 4096 } },
        { id: 'c1', target: 'C:\\Users\\alice\\AppData\\Roaming\\Acme', kind: 'folder', verdict: 'review',
          discoveredBy: ['name equals the publisher'], ownership: [], shared: ['publisher shared with 1 other installed program(s)'],
          userData: [], concerns: [], protection: null, stats: null },
        { id: 'c2', target: 'C:\\Program Files\\Acme', kind: 'folder', verdict: 'blocked', discoveredBy: ['name equals the publisher'],
          ownership: [], shared: [], userData: [], concerns: [], protection: 'contains protected area: installed program: Acme Paint', stats: null },
      ],
    };
    const opFinished = {
      opId: 'op-1', programName: 'Acme Notes', state: 'finished', pids: [], trackingIncomplete: false,
      outcome: scenario === 'success'
        ? { kind: 'succeeded', label: 'Uninstaller finished', detail: '', autoScanAllowed: true }
        : { kind: 'cancelled', label: 'Uninstall was cancelled', detail: '', autoScanAllowed: false },
    };

    const handlers: Record<string, (a: any) => unknown> = {
      env_status: () => ({ stage: 'A', cleanupEnabled: false, restorePoint: 'unavailable', elevated: false, supportedPlatform: true, inventoryUnreadable: [] }),
      list_programs: () => [program('m64:AcmeNotes', 'Acme Notes'), program('m64:AcmePaint', 'Acme Paint')],
      preview_uninstall: (a) => ({
        program: program(a.programId, 'Acme Notes'), installDir: 'C:\\Program Files\\Acme\\Notes', installDirRejected: null,
        commandSummary: '"C:\\Program Files\\Acme\\Notes\\unins000.exe" /SILENT', commandError: null,
        plan: { verb: 'runas', file: 'C:\\Program Files\\Acme\\Notes\\unins000.exe', params: '/SILENT',
          reason: 'Per-machine program; Windows will ask for administrator approval for the uninstaller only.',
          elevation: 'Windows will ask for administrator approval (for the uninstaller only)' },
        refusal: null, restorePoint: 'unavailable', restorePointNote: 'Unavailable in this build.', canLaunch: true,
      }),
      start_uninstall: () => {
        setTimeout(() => emit('uninstall-update', opFinished), 50);
        return 'op-1';
      },
      get_operation: () => ({ ...opFinished, state: 'running', pids: [1234], outcome: null }),
      start_scan: (a) => {
        if (scenario !== 'success' && !a.acknowledgeUncertain) throw 'confirm required';
        setTimeout(() => emit('scan-done', { scanId: 'scan-1', phase: 'Done', itemsVisited: 42, candidateCount: 3, done: true, report }), 50);
        return 'scan-1';
      },
      get_scan: () => ({ scanId: 'scan-1', phase: 'Scanning Program Files', itemsVisited: 10, candidateCount: 1, done: false, report: null }),
      cancel_scan: () => null,
      stop_waiting: () => null,
      'plugin:event|listen': (a) => {
        (listeners[a.event] ??= []).push(a.handler);
        return a.handler;
      },
      'plugin:event|unlisten': () => null,
    };

    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: (cb: (e: unknown) => void) => {
        const id = next++;
        callbacks.set(id, cb);
        return id;
      },
      invoke: async (cmd: string, args: unknown) => {
        calls.push({ cmd, args });
        const h = handlers[cmd];
        if (!h) throw `unmocked command ${cmd}`;
        return h(args);
      },
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
    };
  }, scenario);
}

test('success path: uninstall, auto read-only scan, evidence review without delete controls', async ({ page }) => {
  await mockBackend(page, 'success');
  await page.goto('/');
  await expect(page.getByTestId('stage-banner')).toContainText('disabled in this build');
  await expect(page.getByTestId('program-row')).toHaveCount(2);
  await page.getByRole('button', { name: 'Acme Notes' }).click();

  await expect(page.getByTestId('restore-point')).toContainText('unavailable');
  const run = page.getByRole('button', { name: 'Run vendor uninstaller' });
  await expect(run).toBeDisabled();
  await page.getByLabel(/vendor's uninstaller will run/).check();
  await run.click();

  // Success → scan starts automatically → review.
  await expect(page.getByTestId('cleanup-disabled')).toBeVisible();
  await expect(page.getByTestId('coverage-incomplete')).toContainText('access denied');
  await expect(page.getByTestId('group-likely')).toContainText('C:\\Program Files\\Acme\\Notes');
  await expect(page.getByTestId('group-review')).toContainText('AppData\\Roaming\\Acme');
  await expect(page.getByTestId('group-blocked')).toContainText('C:\\Program Files\\Acme');

  // Expand evidence for a name-only candidate.
  await page.getByRole('button', { name: /AppData\\Roaming\\Acme/ }).click();
  await expect(page.getByTestId('group-review')).toContainText('none (name match only)');

  // No destructive controls anywhere.
  await expect(page.getByRole('button', { name: /delete|remove|clean/i })).toHaveCount(0);
  await expect(page.getByRole('checkbox')).toHaveCount(0);

  // The frontend only ever sent IDs, never paths or commands.
  const calls = await page.evaluate(() => (window as any).__calls as Array<{ cmd: string; args: any }>);
  const sent = JSON.stringify(calls.filter((c) => !c.cmd.startsWith('plugin:')).map((c) => c.args));
  expect(sent).not.toMatch(/\\\\|\.exe|C:/);
});

test('cancelled uninstall: no automatic scan; explicit acknowledgement required', async ({ page }) => {
  await mockBackend(page, 'cancelled');
  await page.goto('/');
  await page.getByRole('button', { name: 'Acme Notes' }).click();
  await page.getByLabel(/vendor's uninstaller will run/).check();
  await page.getByRole('button', { name: 'Run vendor uninstaller' }).click();

  await expect(page.getByTestId('outcome')).toContainText('cancelled');
  const scanAnyway = page.getByRole('button', { name: 'Scan anyway (read-only)' });
  await expect(scanAnyway).toBeDisabled();
  const scansBefore = await page.evaluate(() => (window as any).__calls.filter((c: any) => c.cmd === 'start_scan').length);
  expect(scansBefore).toBe(0);
  await page.getByLabel(/Scan anyway; I understand/).check();
  await scanAnyway.click();
  await expect(page.getByText('The uninstall outcome was not a confirmed success')).toBeVisible();
});
