<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type EnvStatus, type Program } from './api';
  import ProgramList from './ProgramList.svelte';
  import ProgramPanel from './ProgramPanel.svelte';
  import Review from './Review.svelte';

  let env = $state<EnvStatus | null>(null);
  let programs = $state<Program[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selected = $state<string | null>(null);
  let scanId = $state<string | null>(null);

  async function refresh() {
    loading = true;
    error = null;
    try {
      programs = await api.listPrograms();
      env = await api.envStatus();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(refresh);

  function back() {
    scanId = null;
    selected = null;
    refresh();
  }
</script>

<header>
  <h1>Sweep</h1>
  <span class="muted">Stage {env?.stage ?? 'A'} · evidence-based uninstaller</span>
</header>

<main>
  <div class="banner info" data-testid="stage-banner">
    Sweep's own cleanup is <strong>disabled in this build</strong>. Leftover scans are read-only.
    Vendor uninstallers you start <strong>do run and change your system</strong>.
  </div>
  {#if env?.elevated}
    <div class="banner warn">Sweep is running as administrator. Restart it normally: per-user uninstallers are refused while elevated.</div>
  {/if}
  {#if env && !env.supportedPlatform}
    <div class="banner bad">This platform is not supported; Sweep only works on Windows.</div>
  {/if}
  {#if env && env.inventoryUnreadable.length > 0}
    <div class="banner warn">Some registry locations could not be read, so the list may be incomplete:
      <ul>{#each env.inventoryUnreadable as u}<li class="mono">{u}</li>{/each}</ul>
    </div>
  {/if}
  {#if error}<div class="banner bad">{error}</div>{/if}

  {#if scanId}
    <Review {scanId} onBack={back} />
  {:else if selected}
    <ProgramPanel programId={selected} onBack={back} onScan={(id) => (scanId = id)} />
  {:else}
    <ProgramList {programs} {loading} onSelect={(id) => (selected = id)} onRefresh={refresh} />
  {/if}
</main>

<style>
  header { display: flex; align-items: baseline; gap: 12px; padding: 12px 20px; border-bottom: 1px solid var(--border); background: var(--panel); }
  h1 { font-size: 18px; margin: 0; }
  main { padding: 14px 20px; max-width: 1200px; margin: 0 auto; }
</style>
