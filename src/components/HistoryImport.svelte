<script lang="ts">
  import { untrack } from 'svelte';
  import { Upload, ChevronLeft, ChevronRight } from '@lucide/svelte';
  import { call, errorMessage } from '../lib/api';
  import type { ImportedPlay, Track } from '../lib/types';

  let {
    count,
    play,
    run,
  }: {
    count: number;
    play: (track: Track) => void;
    run: (action: () => Promise<void>, success?: string) => Promise<boolean>;
  } = $props();
  let entries = $state<ImportedPlay[]>([]);
  let offset = $state(0);
  let pending = $state(false);
  let message = $state('');
  let input: HTMLInputElement;

  async function loadPage(start: number) {
    pending = true;
    try {
      entries = await call<ImportedPlay[]>('imported_history', {
        offset: start,
      });
      offset = start;
    } catch (error) {
      message = errorMessage(error);
    } finally {
      pending = false;
    }
  }
  $effect(() => {
    if (count >= 0)
      untrack(() => {
        void loadPage(0);
      });
  });

  async function importFile(file: File | undefined) {
    if (!file) return;
    if (file.size > 20 * 1024 * 1024) {
      message = 'Choose a JSON file smaller than 20 MB.';
      return;
    }
    pending = true;
    try {
      await run(async () => {
        const result = await call<{
          imported: number;
          duplicates: number;
          skipped: number;
        }>('import_history', { contents: await file.text() });
        message = `${result.imported} events imported; ${result.duplicates} duplicates and ${result.skipped} unsupported entries skipped.`;
      });
    } finally {
      pending = false;
      input.value = '';
    }
  }
</script>

<div class="section-heading">
  <h2>Imported Google history</h2>
  <button disabled={pending} onclick={() => input.click()}
    ><Upload size={16} />Import JSON</button
  >
</div>
<input
  class="sr-only"
  tabindex="-1"
  type="file"
  accept=".json,application/json"
  aria-label="Google Takeout history file"
  bind:this={input}
  onchange={(event) => importFile(event.currentTarget.files?.[0])}
/>
<p class="help">
  Export YouTube watch history as JSON from Google Takeout, then select
  watch-history.json. The file stays on this device. Imported events have no
  listening duration and are excluded from listening-time statistics.
</p>
{#if message}<p role="status">{message}</p>{/if}
{#if entries.length}
  <div class="history-list">
    {#each entries as entry, index (`${entry.track.id}:${entry.playedAt}`)}<button
        class="history-row"
        onclick={() => play(entry.track)}
        ><span class="muted">{offset + index + 1}</span>
        <div>
          <strong>{entry.track.title}</strong><span>{entry.track.artist}</span>
        </div>
        <span>Imported</span><small
          >{new Date(entry.playedAt).toLocaleDateString()}</small
        ></button
      >{/each}
  </div>
{:else}<p>No imported watch events yet.</p>{/if}
<div class="section-heading section-footnote">
  <span>{count} imported events</span>
  <div class="button-row">
    <button
      disabled={pending || offset === 0}
      aria-label="Previous history page"
      onclick={() => loadPage(Math.max(0, offset - 100))}
      ><ChevronLeft size={16} /></button
    ><button
      disabled={pending || offset + entries.length >= count}
      aria-label="Next history page"
      onclick={() => loadPage(offset + 100)}><ChevronRight size={16} /></button
    >
  </div>
</div>
