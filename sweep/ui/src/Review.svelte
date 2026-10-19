<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, formatBytes, onScanDone, onScanProgress, type CandidateView, type ScanView } from './api';

  let { scanId, onBack }: { scanId: string; onBack: () => void } = $props();

  let view = $state<ScanView | null>(null);
  let error = $state<string | null>(null);
  let showAllCoverage = $state(false);
  let expanded = $state<Record<string, boolean>>({});
  const unlistens: Array<() => void> = [];

  onMount(async () => {
    unlistens.push(await onScanProgress((v) => { if (v.scanId === scanId && !view?.done) view = v; }));
    unlistens.push(await onScanDone((v) => { if (v.scanId === scanId) view = v; }));
    try {
      const v = await api.getScan(scanId);
      if (!view?.done) view = v;
    } catch (e) {
      error = String(e);
    }
  });
  onDestroy(() => unlistens.forEach((u) => u()));

  const groups: Array<{ key: CandidateView['verdict']; title: string; note: string }> = [
    { key: 'likely', title: 'Likely leftovers', note: 'Strong ownership evidence and no shared-use, user-data or coverage concerns.' },
    { key: 'review', title: 'Needs review', note: 'Found by name or with concerns. Never preselected.' },
    { key: 'blocked', title: 'Protected', note: 'System or shared locations. Can never be selected.' },
  ];
  const report = $derived(view?.report ?? null);
  const gaps = $derived(report ? report.coverage.filter((c) => c.gap) : []);
</script>

<button onclick={onBack}>← Programs</button>

{#if error}<div class="banner bad top">{error}</div>{/if}

{#if view && !view.done}
  <div class="panel top" data-testid="scan-progress">
    <h3>Scanning (read-only)…</h3>
    <progress></progress>
    <p class="muted">{view.phase} · {view.itemsVisited} items examined · {view.candidateCount} candidates so far</p>
    <button onclick={() => api.cancelScan(scanId)}>Cancel scan</button>
  </div>
{/if}

{#if report}
  <div class="top">
    <div class="banner info" data-testid="cleanup-disabled">
      <strong>Cleanup is disabled in this build.</strong> This is a review of evidence only; nothing will be deleted.
    </div>
    {#if report.uncertain}
      <div class="banner warn">The uninstall outcome was not a confirmed success. The program may still be installed.</div>
    {/if}
    {#if report.cancelled}
      <div class="banner warn" data-testid="cancelled">Scan was cancelled. Results are partial.</div>
    {/if}
    {#if report.complete}
      <div class="banner ok" data-testid="coverage-complete">All planned locations were examined ({report.itemsVisited} items).</div>
    {:else}
      <div class="banner bad" data-testid="coverage-incomplete">
        <strong>Coverage incomplete:</strong> {gaps.length} location(s) could not be fully examined.
        {#if report.candidates.length === 0}An empty result here does <em>not</em> mean nothing was left behind.{/if}
        <ul>
          {#each gaps as g}<li><code>{g.location}</code> — {g.status}{g.detail ? `: ${g.detail}` : ''}</li>{/each}
        </ul>
      </div>
    {/if}
  </div>

  {#each groups as g}
    {@const items = report.candidates.filter((c) => c.verdict === g.key)}
    <section class="panel" data-testid="group-{g.key}">
      <h3>{g.title} <span class="badge {g.key}">{items.length}</span></h3>
      <p class="muted">{g.note}</p>
      {#if items.length === 0}<p class="muted">None.</p>{/if}
      {#each items as c (c.id)}
        <div class="cand">
          <button class="rowbtn" onclick={() => (expanded[c.id] = !expanded[c.id])} aria-expanded={!!expanded[c.id]}>
            <span class="kind">{c.kind}</span>
            <code>{c.target}</code>
            {#if c.stats && c.kind !== 'registry'}<span class="muted">· {c.stats.files} files, {formatBytes(c.stats.bytes)}</span>{/if}
          </button>
          {#if expanded[c.id]}
            <ul class="why">
              {#each c.discoveredBy as d}<li><strong>Found because:</strong> {d}</li>{/each}
              {#each c.ownership as o}<li class="good"><strong>Ownership evidence:</strong> {o}</li>{/each}
              {#if c.ownership.length === 0}<li><strong>Ownership evidence:</strong> none (name match only)</li>{/if}
              {#each c.shared as s}<li class="warn"><strong>Possibly shared:</strong> {s}</li>{/each}
              {#each c.userData as u}<li class="warn"><strong>Possible user data:</strong> {u}</li>{/each}
              {#each c.concerns as k}<li class="warn"><strong>Coverage concern:</strong> {k}</li>{/each}
              {#if c.protection}<li class="bad"><strong>Protected:</strong> {c.protection}</li>{/if}
            </ul>
          {/if}
        </div>
      {/each}
    </section>
  {/each}

  <section class="panel">
    <button onclick={() => (showAllCoverage = !showAllCoverage)}>{showAllCoverage ? 'Hide' : 'Show'} all {report.coverage.length} examined locations</button>
    {#if showAllCoverage}
      <ul>{#each report.coverage as c}<li><code>{c.location}</code> — {c.status}</li>{/each}</ul>
    {/if}
  </section>
{/if}

<style>
  .top { margin-top: 12px; }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .cand { border-top: 1px solid var(--border); padding: 6px 0; }
  .rowbtn { border: none; background: none; padding: 2px 0; text-align: left; width: 100%; display: flex; gap: 8px; align-items: baseline; }
  .kind { font-size: 11px; text-transform: uppercase; color: var(--muted); width: 64px; flex: none; }
  .why { margin: 4px 0 4px 72px; padding-left: 16px; }
  .why li { margin: 2px 0; }
  .good { color: var(--ok); }
  .warn { color: var(--warn); }
  .bad { color: var(--bad); }
</style>
