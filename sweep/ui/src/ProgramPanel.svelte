<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, formatDate, onUninstallUpdate, type OpView, type Preview } from './api';

  let { programId, onBack, onScan }: { programId: string; onBack: () => void; onScan: (scanId: string) => void } = $props();

  let preview = $state<Preview | null>(null);
  let error = $state<string | null>(null);
  let understood = $state(false);
  let ackUncertain = $state(false);
  let ackAnalyze = $state(false);
  let op = $state<OpView | null>(null);
  let busy = $state(false);
  let unlisten: (() => void) | null = null;
  let autoScanStarted = false;

  onMount(async () => {
    try {
      preview = await api.previewUninstall(programId);
    } catch (e) {
      error = String(e);
    }
    unlisten = await onUninstallUpdate((v) => {
      if (op && v.opId === op.opId) {
        op = v;
        maybeAutoScan();
      }
    });
  });
  onDestroy(() => unlisten?.());

  async function maybeAutoScan() {
    if (op?.state === 'finished' && op.outcome?.autoScanAllowed && !autoScanStarted) {
      autoScanStarted = true;
      await scan(false);
    }
  }

  async function run() {
    busy = true;
    error = null;
    try {
      const opId = await api.startUninstall(programId);
      op = await api.getOperation(opId);
      maybeAutoScan();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function scan(ack: boolean) {
    try {
      const id = op ? await api.startScan({ opId: op.opId }, ack) : await api.startScan({ programId }, ack);
      onScan(id);
    } catch (e) {
      error = String(e);
    }
  }

  const stateLabel: Record<string, string> = {
    ready: 'Preparing', launching: 'Starting the uninstaller…', running: 'Uninstaller running', verifying: 'Checking the result…', finished: 'Finished',
  };
</script>

<button onclick={onBack}>← Programs</button>

{#if error}<div class="banner bad top">{error}</div>{/if}

{#if preview}
  {@const p = preview.program}
  <div class="panel top">
    <h2>{p.name}</h2>
    <dl>
      <dt>Publisher</dt><dd>{p.publisher ?? '—'}</dd>
      <dt>Version</dt><dd>{p.version ?? '—'}</dd>
      <dt>Installed for</dt><dd>{p.scope} ({p.views.join(', ')})</dd>
      <dt>Install date</dt><dd>{formatDate(p.installDate) || '—'}</dd>
      <dt>Install folder</dt>
      <dd>
        {#if preview.installDir}<code>{preview.installDir}</code>
        {:else}<span class="muted">not used as evidence ({preview.installDirRejected})</span>{/if}
      </dd>
      <dt>Uninstall command</dt>
      <dd>
        {#if preview.commandSummary}<code>{preview.commandSummary}</code>
        {:else}<span class="bad-text">Not supported: {preview.commandError}</span>{/if}
      </dd>
    </dl>
  </div>

  {#if !op}
    <div class="panel">
      <h3>Before you uninstall</h3>
      <div class="banner warn" data-testid="restore-point">
        <strong>Restore point: unavailable.</strong> {preview.restorePointNote}
      </div>
      {#if preview.plan}
        <p><strong>How it will run:</strong> {preview.plan.reason}</p>
        <p class="muted">Elevation: {preview.plan.elevation}. Sweep itself stays unelevated.</p>
        <label><input type="checkbox" bind:checked={understood} />
          I understand the vendor's uninstaller will run and change my system. Sweep cannot undo it.</label>
        <div class="actions">
          <button class="primary" disabled={!understood || busy || !preview.canLaunch} onclick={run}>Run vendor uninstaller</button>
        </div>
      {:else}
        <div class="banner bad">Sweep will not start this uninstaller: {preview.refusal}</div>
      {/if}
      <hr />
      <p class="muted">Or look for related files and registry keys without uninstalling. Results describe the current installation and are read-only.</p>
      <label><input type="checkbox" bind:checked={ackAnalyze} /> The program is still installed; I understand results are only indicative.</label>
      <div class="actions"><button disabled={!ackAnalyze} onclick={() => scan(true)}>Analyze without uninstalling</button></div>
    </div>
  {:else}
    <div class="panel" data-testid="operation">
      <h3>{stateLabel[op.state]}</h3>
      {#if op.state === 'running' || op.state === 'launching'}
        <progress></progress>
        <p class="muted">Follow the uninstaller's own window. Tracking {op.pids.length} process(es).</p>
        <button onclick={() => api.stopWaiting(op!.opId)}>Stop waiting</button>
        <span class="muted">Use this if the uninstaller opened something that stays open (e.g. a browser).</span>
      {/if}
      {#if op.trackingIncomplete}
        <div class="banner warn">Some uninstaller processes could not be tracked; the result is based on checks after they exited.</div>
      {/if}
      {#if op.outcome}
        <div class="banner {op.outcome.kind === 'succeeded' ? 'ok' : op.outcome.kind === 'failed' ? 'bad' : 'warn'}" data-testid="outcome">
          <strong>{op.outcome.label}.</strong> {op.outcome.detail}
        </div>
        {#if op.outcome.autoScanAllowed}
          <p class="muted">Starting the read-only leftover scan…</p>
        {:else}
          <p>Because the outcome is not a confirmed success, Sweep does not scan automatically.</p>
          <label><input type="checkbox" bind:checked={ackUncertain} /> Scan anyway; I understand the program may still be installed.</label>
          <div class="actions"><button disabled={!ackUncertain} onclick={() => scan(true)}>Scan anyway (read-only)</button></div>
        {/if}
      {/if}
    </div>
  {/if}
{:else if !error}
  <p class="muted top">Loading…</p>
{/if}

<style>
  .top { margin-top: 12px; }
  h2 { margin: 0 0 8px; font-size: 18px; }
  h3 { margin: 0 0 10px; font-size: 15px; }
  dl { display: grid; grid-template-columns: 140px 1fr; gap: 4px 12px; margin: 0; }
  dt { color: var(--muted); }
  dd { margin: 0; }
  .actions { margin-top: 10px; display: flex; gap: 8px; }
  .bad-text { color: var(--bad); }
  hr { border: none; border-top: 1px solid var(--border); margin: 16px 0; }
  label { display: block; margin-top: 6px; }
</style>
