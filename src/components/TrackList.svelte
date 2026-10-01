<script lang="ts">
  import {
    Play,
    Ellipsis,
    ExternalLink,
    Trash2,
    ListPlus,
  } from '@lucide/svelte';
  import type { Track } from '../lib/types';
  let {
    tracks,
    currentId,
    play,
    remove,
    addTo,
    open,
  }: {
    tracks: Track[];
    currentId?: string;
    play: (track: Track) => void;
    remove?: (track: Track) => void;
    addTo: (track: Track) => void;
    open: (track: Track) => void;
  } = $props();
  let menu = $state<number | null>(null);
</script>

<div class="track-table" role="table" aria-label="Tracks">
  <div class="track-head" role="row">
    <span role="columnheader">#</span><span role="columnheader">Title</span
    ><span role="columnheader">Artist / channel</span><span
      role="columnheader"
      class="sr-only">Actions</span
    >
  </div>
  {#each tracks as track, index (`${track.itemId ?? track.id}:${index}`)}
    <div
      class:current={currentId === track.id}
      class="track-row"
      role="row"
      tabindex="0"
      oncontextmenu={(event) => {
        event.preventDefault();
        menu = index;
      }}
    >
      <div role="cell">
        <button
          class="track-play icon-button"
          aria-label={`Play ${track.title}`}
          onclick={() => play(track)}
          ><span class="track-number">{index + 1}</span><Play
            size={14}
          /></button
        >
      </div>
      <div class="track-title" role="cell">
        <img
          src={`https://i.ytimg.com/vi/${track.id}/default.jpg`}
          alt=""
          loading="lazy"
        /><button onclick={() => play(track)}>{track.title}</button>
      </div>
      <span class="track-artist" role="cell"
        >{track.artist || 'Unknown artist'}</span
      >
      <div class="track-actions" role="cell">
        <button
          class="icon-button"
          aria-label={`Actions for ${track.title}`}
          aria-expanded={menu === index}
          onclick={() => (menu = menu === index ? null : index)}
          ><Ellipsis size={18} /></button
        >
        {#if menu === index}
          <div class="context-menu">
            <button
              onclick={() => {
                menu = null;
                addTo(track);
              }}><ListPlus size={15} />Add to playlist</button
            >
            <button
              onclick={() => {
                menu = null;
                open(track);
              }}><ExternalLink size={15} />Open in YouTube Music</button
            >
            {#if remove}<button
                class="danger-text"
                onclick={() => {
                  menu = null;
                  remove(track);
                }}><Trash2 size={15} />Remove from playlist</button
              >{/if}
            <button onclick={() => (menu = null)}>Close menu</button>
          </div>
        {/if}
      </div>
    </div>
  {/each}
</div>
