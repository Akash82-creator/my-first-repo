<script lang="ts">
  import { formatDate, formatSize, type Program } from './api';

  let { programs, loading, onSelect, onRefresh }: {
    programs: Program[]; loading: boolean; onSelect: (id: string) => void; onRefresh: () => void;
  } = $props();

  let query = $state('');
  let sortKey = $state<'name' | 'publisher' | 'size' | 'date'>('name');

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const list = programs.filter((p) => !q || p.name.toLowerCase().includes(q) || (p.publisher ?? '').toLowerCase().includes(q));
    const cmp: Record<string, (a: Program, b: Program) => number> = {
      name: (a, b) => a.name.localeCompare(b.name),
      publisher: (a, b) => (a.publisher ?? '').localeCompare(b.publisher ?? ''),
      size: (a, b) => (b.sizeKb ?? 0) - (a.sizeKb ?? 0),
      date: (a, b) => (b.installDate ?? '').localeCompare(a.installDate ?? ''),
    };
    return [...list].sort(cmp[sortKey]);
  });
</script>

<div class="toolbar">
  <input type="search" placeholder="Search programs or publishers" bind:value={query} aria-label="Search" />
  <label class="muted">Sort
    <select bind:value={sortKey}>
      <option value="name">Name</option>
      <option value="publisher">Publisher</option>
      <option value="size">Size</option>
      <option value="date">Install date</option>
    </select>
  </label>
  <button onclick={onRefresh} disabled={loading}>Refresh</button>
  <span class="muted">{shown.length} of {programs.length} programs</span>
</div>

{#if loading}
  <p class="muted">Reading installed programs…</p>
{:else if programs.length === 0}
  <p class="muted">No programs found.</p>
{:else}
  <table>
    <thead><tr><th>Name</th><th>Publisher</th><th>Version</th><th>Installed for</th><th>Size</th><th>Date</th></tr></thead>
    <tbody>
      {#each shown as p (p.id)}
        <tr onclick={() => onSelect(p.id)} data-testid="program-row">
          <td>
            <button class="link" onclick={(e) => { e.stopPropagation(); onSelect(p.id); }}>{p.name}</button>
            {#if p.msi}<span class="badge">MSI</span>{/if}
            {#each p.views as v}<span class="badge">{v}</span>{/each}
            {#if !p.hasUninstaller}<span class="badge">no uninstaller</span>{/if}
          </td>
          <td>{p.publisher ?? ''}</td>
          <td>{p.version ?? ''}</td>
          <td>{p.scope}</td>
          <td>{formatSize(p.sizeKb)}</td>
          <td>{formatDate(p.installDate)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .toolbar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-bottom: 10px; }
  select { font: inherit; margin-left: 4px; background: var(--panel); color: var(--text); border: 1px solid var(--border); border-radius: 4px; }
  table { width: 100%; border-collapse: collapse; background: var(--panel); border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
  th, td { text-align: left; padding: 7px 10px; border-bottom: 1px solid var(--border); }
  th { font-weight: 600; color: var(--muted); font-size: 12px; }
  tbody tr { cursor: pointer; }
  tbody tr:hover { background: var(--row-hover); }
  button.link { border: none; background: none; padding: 0; color: var(--text); font-weight: 600; text-align: left; }
</style>
