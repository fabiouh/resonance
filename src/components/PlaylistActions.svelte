<script lang="ts">
  import { Pencil, Trash2 } from '@lucide/svelte';
  import { call } from '../lib/api';
  import type { Playlist } from '../lib/types';
  import Modal from './Modal.svelte';

  let {
    playlist,
    run,
    deleted,
  }: {
    playlist: Playlist;
    run: (action: () => Promise<void>, success?: string) => Promise<boolean>;
    deleted: () => void;
  } = $props();
  let action = $state<'rename' | 'delete' | null>(null);
  let name = $state('');
  let confirmation = $state('');
  let pending = $state(false);

  async function submit() {
    pending = true;
    try {
      const success =
        action === 'rename'
          ? await run(
              () => call('rename_playlist', { id: playlist.id, name }),
              'Playlist renamed',
            )
          : await run(
              () => call('delete_playlist', { id: playlist.id, confirmation }),
              'Playlist deleted',
            );
      if (success) {
        if (action === 'delete') deleted();
        action = null;
      }
    } finally {
      pending = false;
    }
  }
</script>

<button
  class="icon-button"
  aria-label="Rename playlist"
  onclick={() => {
    name = playlist.name;
    action = 'rename';
  }}><Pencil size={17} /></button
>
<button
  class="icon-button"
  aria-label="Delete playlist"
  onclick={() => {
    confirmation = '';
    action = 'delete';
  }}><Trash2 size={17} /></button
>

{#if action}
  <Modal
    title={action === 'rename' ? 'Rename playlist' : 'Delete playlist?'}
    close={() => {
      if (!pending) action = null;
    }}
  >
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      {#if action === 'rename'}
        <label>Name<input bind:value={name} required maxlength="150" /></label>
        {#if playlist.remoteId}<p class="help">
            The name will also change on YouTube. Your playlist description and
            visibility will be preserved.
          </p>{/if}
      {:else}
        <p>
          Delete “{playlist.name}”? {playlist.remoteId
            ? 'This permanently deletes the playlist from your YouTube account.'
            : 'This removes the local playlist from this device.'}
        </p>
        {#if playlist.remoteId}<label
            >Type the playlist name to confirm<input
              bind:value={confirmation}
              autocomplete="off"
              required
            /></label
          >{/if}
        <p class="help">
          Other playlists are unaffected. Remove any curation rules referencing
          this playlist first.
        </p>
      {/if}
      <div class="dialog-actions">
        <button type="button" disabled={pending} onclick={() => (action = null)}
          >Cancel</button
        >
        <button
          class:primary={action === 'rename'}
          class:danger={action === 'delete'}
          disabled={pending ||
            (action === 'delete' &&
              !!playlist.remoteId &&
              confirmation !== playlist.name)}
          >{action === 'rename' ? 'Save name' : 'Delete playlist'}</button
        >
      </div>
    </form>
  </Modal>
{/if}
