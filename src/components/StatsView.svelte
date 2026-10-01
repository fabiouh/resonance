<script lang="ts">
  import { Clock3, Music2, Trash2 } from '@lucide/svelte';
  import type { Listening, Track } from '../lib/types';
  import { duration } from '../lib/library';
  import Modal from './Modal.svelte';
  import { call } from '../lib/api';
  let {
    history,
    play,
    run,
  }: {
    history: Record<string, Listening>;
    play: (track: Track) => void;
    run: (action: () => Promise<void>, success?: string) => Promise<boolean>;
  } = $props();
  let confirm = $state(false);
  let entries = $derived(
    Object.values(history).sort((a, b) => b.seconds - a.seconds),
  );
  let total = $derived(entries.reduce((sum, entry) => sum + entry.seconds, 0));
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">ON REPEAT</div>
    <h1>Listening history</h1>
    <p>A local record of the music you play in Resonance.</p>
  </div>
  <button disabled={!entries.length} onclick={() => (confirm = true)}
    ><Trash2 size={16} />Clear history</button
  >
</div>
{#if entries.length}
  <div class="listening-summary">
    <div>
      <Clock3 size={22} /><strong>{duration(total)}</strong><span>listened</span
      >
    </div>
    <div>
      <Music2 size={22} /><strong>{entries.length}</strong><span
        >unique tracks</span
      >
    </div>
  </div>
  <h2 class="section-title">Most listened</h2>
  <div class="history-list">
    {#each entries as entry, index (entry.track.id)}<button
        class="history-row"
        onclick={() => play(entry.track)}
        ><span class="muted">{index + 1}</span>
        <div>
          <strong>{entry.track.title}</strong><span>{entry.track.artist}</span>
        </div>
        <span>{duration(entry.seconds)}</span><small
          >{new Date(entry.lastPlayed * 1000).toLocaleDateString()}</small
        ></button
      >{/each}
  </div>
{:else}<div class="empty-state">
    <Clock3 size={38} />
    <h2>No listening history yet</h2>
    <p>Play a track here to start your local listening record.</p>
  </div>{/if}
<p class="help section-footnote">
  Only time played in the embedded player is recorded, in short intervals.
  Playback in your browser or another app is not counted.
</p>
{#if confirm}<Modal
    title="Clear listening history?"
    close={() => (confirm = false)}
    ><p>
      This removes all recorded listening time from this device. Your playlists
      will remain.
    </p>
    <div class="dialog-actions">
      <button onclick={() => (confirm = false)}>Cancel</button><button
        class="danger"
        onclick={async () => {
          if (
            await run(() => call('clear_history'), 'Listening history cleared')
          )
            confirm = false;
        }}>Clear history</button
      >
    </div></Modal
  >{/if}
