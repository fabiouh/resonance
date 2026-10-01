<script lang="ts">
  import { onMount, setContext } from 'svelte';
  import { version } from '../package.json';
  import { listen } from '@tauri-apps/api/event';
  import { check } from '@tauri-apps/plugin-updater';
  import {
    AudioLines,
    Library,
    ListMusic,
    GitMerge,
    Clock3,
    Settings,
    Search,
    Plus,
    RefreshCw,
    ArrowUpRight,
    Music2,
    X,
    Trash2,
  } from '@lucide/svelte';
  import { call, desktop, errorMessage, snapshot } from './lib/api';
  import { allTracks, filterTracks, videoId } from './lib/library';
  import type { Snapshot, Track, Playlist } from './lib/types';
  import TrackList from './components/TrackList.svelte';
  import Player from './components/Player.svelte';
  import Modal from './components/Modal.svelte';
  import RulesView from './components/RulesView.svelte';
  import SettingsView from './components/SettingsView.svelte';
  import StatsView from './components/StatsView.svelte';

  let data = $state<Snapshot | null>(null);
  let page = $state('library');
  let query = $state('');
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');
  setContext('feedback', { error: () => error });
  let current = $state<Track | null>(null);
  let queue = $state<Track[]>([]);
  let createOpen = $state(false);
  let playlistName = $state('');
  let remote = $state(false);
  let adding = $state(false);
  let addId = $state('');
  let trackUrl = $state('');
  let removing = $state<Track | null>(null);
  let deleting = $state<Playlist | null>(null);
  let searchInput: HTMLInputElement;
  let selected = $derived(data?.library.playlists.find((p) => p.id === page));
  let tracks = $derived(
    filterTracks(
      selected?.tracks ?? allTracks(data?.library.playlists ?? []),
      query,
    ),
  );
  let libraryPage = $derived(page === 'library' || !!selected);
  let count = $derived(allTracks(data?.library.playlists ?? []).length);

  function report(message: string) {
    error = message;
  }
  async function refresh() {
    data = await snapshot();
  }
  async function run(
    action: () => Promise<void>,
    success = '',
  ): Promise<boolean> {
    if (busy) return false;
    busy = true;
    error = '';
    notice = '';
    try {
      await action();
      await refresh();
      notice = success;
      return true;
    } catch (reason) {
      error = errorMessage(reason);
      try {
        await refresh();
      } catch {
        /* Preserve the actionable operation error. */
      }
      return false;
    } finally {
      busy = false;
    }
  }
  function navigate(destination: string) {
    page = destination;
    query = '';
    if (destination === 'history')
      void refresh().catch((reason) => report(errorMessage(reason)));
  }
  function play(track: Track) {
    queue = tracks.some((t) => t.id === track.id) ? [...tracks] : [track];
    current = track;
  }
  function next() {
    const index = queue.findIndex((t) => t.id === current?.id);
    if (index >= 0 && index + 1 < queue.length) current = queue[index + 1];
  }
  function previous() {
    const index = queue.findIndex((t) => t.id === current?.id);
    if (index > 0) current = queue[index - 1];
  }
  function addTo(track?: Track) {
    addId = selected?.id ?? data?.library.playlists[0]?.id ?? '';
    trackUrl = track?.id ?? '';
    adding = true;
  }

  onMount(() => {
    if (!desktop) return;
    void refresh()
      .then(async () => {
        if (!data?.updaterConfigured) return;
        try {
          const update = await check();
          if (update) {
            notice = `Resonance ${update.version} is available. Open Settings to install it.`;
            await update.close();
          }
        } catch {
          notice = 'Could not check for updates. Try again in Settings.';
        }
      })
      .catch((reason) => report(errorMessage(reason)));
    const unsubscribe = listen('library-changed', () => {
      void refresh().catch((reason) => report(errorMessage(reason)));
    });
    const keyboard = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        navigate('library');
        searchInput?.focus();
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'n') {
        event.preventDefault();
        createOpen = true;
      }
    };
    window.addEventListener('keydown', keyboard);
    return () => {
      void unsubscribe.then((stop) => stop());
      window.removeEventListener('keydown', keyboard);
    };
  });
</script>

<div class="app-shell" aria-busy={busy}>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark"><AudioLines size={25} /></div>
      <span>resonance<span class="brand-period">.</span></span>
    </div>
    <div class="section-label">YOUR MUSIC</div>
    <nav aria-label="Main navigation">
      <button
        class:active={page === 'library'}
        onclick={() => navigate('library')}
        ><Library size={18} />Library<span class="nav-count">{count}</span
        ></button
      >
      <button class:active={page === 'rules'} onclick={() => navigate('rules')}
        ><GitMerge size={18} />Curation rules</button
      >
      <button
        class:active={page === 'history'}
        onclick={() => navigate('history')}
        ><Clock3 size={18} />Listening history</button
      >
    </nav>
    <div class="playlist-label">
      <span class="section-label">PLAYLISTS</span><button
        class="icon-button"
        aria-label="Create playlist (Ctrl+N)"
        onclick={() => (createOpen = true)}><Plus size={16} /></button
      >
    </div>
    <nav class="playlist-nav" aria-label="Playlists">
      {#each data?.library.playlists ?? [] as playlist (playlist.id)}<button
          class:active={page === playlist.id}
          onclick={() => navigate(playlist.id)}
          ><ListMusic size={17} /><span>{playlist.name}</span><span
            class="nav-count">{playlist.tracks.length}</span
          ></button
        >{/each}
    </nav>
    {#if !data?.library.playlists.length}<p class="sidebar-empty">
        Your playlists will appear here.
      </p>{/if}
    <div class="sidebar-bottom">
      <button
        onclick={() =>
          call('open_youtube', { videoId: null }).catch((reason) =>
            report(errorMessage(reason)),
          )}><ArrowUpRight size={18} />YouTube Music</button
      ><button
        class:active={page === 'settings'}
        onclick={() => navigate('settings')}
        ><Settings size={18} />Settings</button
      >
      <div class="account-line">
        <span class:enabled={data?.connected} class="status-dot"
        ></span>{data?.connected ? 'Google connected' : 'Local library'}
      </div>
    </div>
  </aside>

  <div class="workspace">
    <header class="topbar">
      <label class="search"
        ><Search size={17} /><input
          bind:this={searchInput}
          bind:value={query}
          placeholder="Search your library"
          aria-label="Search your library"
          oninput={() => {
            if (!libraryPage) page = 'library';
          }}
        /><kbd>Ctrl K</kbd></label
      ><button
        disabled={busy || !data}
        onclick={() =>
          run(
            () => call('sync_library', { apply: false }),
            'Library refreshed',
          )}
        ><RefreshCw size={15} class={busy ? 'spinning' : ''} />{busy
          ? 'Working…'
          : 'Sync library'}</button
      >
    </header>
    {#if error}<div class="banner error" role="alert">
        <span>{error}</span><button
          class="icon-button"
          aria-label="Dismiss error"
          onclick={() => (error = '')}><X size={16} /></button
        >
      </div>{/if}
    {#if notice}<div class="banner" role="status">
        <span>{notice}</span><button
          class="icon-button"
          aria-label="Dismiss notification"
          onclick={() => (notice = '')}><X size={16} /></button
        >
      </div>{/if}
    {#if data?.syncError && data.syncError !== error}<div
        class="banner error"
        role="status"
      >
        Last sync: {data.syncError}
      </div>{/if}
    <main>
      {#if !desktop}<div class="empty-state">
          <AudioLines size={40} />
          <h1>Open the desktop application</h1>
          <p>
            Your library uses native storage. Start Resonance with <code
              >pnpm tauri dev</code
            >.
          </p>
        </div>
      {:else if !data}<div class="empty-state">
          <AudioLines size={40} />
          <h1>
            {error ? 'Could not open your library' : 'Opening your library…'}
          </h1>
          {#if error}<button
              onclick={() =>
                refresh().catch((reason) => report(errorMessage(reason)))}
              >Try again</button
            >{/if}
        </div>
      {:else if page === 'rules'}<RulesView library={data.library} {run} />
      {:else if page === 'settings'}<SettingsView
          settings={data.library.settings}
          connected={data.connected}
          updaterConfigured={data.updaterConfigured}
          {run}
          notify={report}
        />
      {:else if page === 'history'}<StatsView
          history={data.library.listening}
          {play}
          {run}
        />
      {:else}
        <div class="page-heading">
          <div>
            <div class="eyebrow">
              {selected
                ? selected.remoteId
                  ? 'YOUTUBE PLAYLIST'
                  : 'LOCAL PLAYLIST'
                : 'COLLECTED, NOT COMPLICATED'}
            </div>
            <h1>{selected?.name ?? 'Your library'}</h1>
            <p>
              {selected
                ? `${selected.tracks.length} tracks`
                : `${count} tracks across ${data.library.playlists.length} playlists`}
            </p>
          </div>
          <div class="button-row">
            {#if selected && !selected.remoteId}<button
                class="icon-button"
                aria-label="Delete playlist"
                onclick={() => (deleting = selected ?? null)}
                ><Trash2 size={17} /></button
              >{/if}<button
              class="primary"
              onclick={() => {
                if (selected) addTo();
                else createOpen = true;
              }}
              ><Plus size={16} />{selected
                ? 'Add track'
                : 'New playlist'}</button
            >
          </div>
        </div>
        {#if page === 'library' && !query && data.library.playlists.length}
          <div class="section-heading">
            <h2>Your playlists</h2>
            <span>{data.library.playlists.length} collections</span>
          </div>
          <div class="playlist-grid">
            {#each data.library.playlists as playlist (playlist.id)}<button
                class="playlist-tile"
                onclick={() => navigate(playlist.id)}
                ><div class="playlist-art">
                  {#if playlist.tracks[0]}<img
                      src={`https://i.ytimg.com/vi/${playlist.tracks[0].id}/hqdefault.jpg`}
                      alt=""
                      loading="lazy"
                    />{:else}<ListMusic size={36} strokeWidth={1.4} />{/if}<span
                    >{playlist.remoteId ? 'YouTube' : 'Local'}</span
                  >
                </div>
                <strong>{playlist.name}</strong><small
                  >{playlist.tracks.length} tracks</small
                ></button
              >{/each}
          </div>
        {/if}
        <div class="section-heading">
          <h2>
            {query ? 'Search results' : selected ? 'Tracks' : 'All tracks'}
          </h2>
          <span>{tracks.length} tracks</span>
        </div>
        {#if tracks.length}<TrackList
            {tracks}
            currentId={current?.id}
            {play}
            {addTo}
            open={(track) =>
              call('open_youtube', { videoId: track.id }).catch((reason) =>
                report(errorMessage(reason)),
              )}
            remove={selected ? (track) => (removing = track) : undefined}
          />
        {:else}<div class="empty-state">
            <Music2 size={38} />
            <h2>
              {query
                ? 'No matching tracks'
                : selected
                  ? 'No tracks yet'
                  : 'Make room for your music'}
            </h2>
            <p>
              {query
                ? 'Try a different title or artist.'
                : selected
                  ? 'Add a YouTube link, or use this playlist as a curation target.'
                  : 'Create a playlist to collect tracks, or connect Google in Settings to sync your library.'}
            </p>
            {#if !query && !selected}<div class="button-row">
                <button class="primary" onclick={() => (createOpen = true)}
                  ><Plus size={16} />Create playlist</button
                ><button onclick={() => navigate('settings')}
                  >Connect Google</button
                >
              </div>{/if}
          </div>{/if}
      {/if}
    </main>
    <div class="workspace-status">
      <span
        >{data?.library.lastSync
          ? `Last synced ${new Date(data.library.lastSync * 1000).toLocaleString()}`
          : 'Not synced yet'}</span
      ><span>Resonance {version}</span>
    </div>
  </div>
  {#if desktop}<Player
      track={current}
      {next}
      {previous}
      {report}
      canPrevious={queue.findIndex((t) => t.id === current?.id) > 0}
      canNext={!!current &&
        queue.findIndex((t) => t.id === current?.id) < queue.length - 1}
    />{/if}
</div>

{#if createOpen}<Modal title="New playlist" close={() => (createOpen = false)}
    ><form
      onsubmit={async (event) => {
        event.preventDefault();
        if (
          await run(
            () => call('create_playlist', { name: playlistName, remote }),
            'Playlist created',
          )
        ) {
          createOpen = false;
          playlistName = '';
          remote = false;
        }
      }}
    >
      <label
        >Name<input
          required
          maxlength="150"
          bind:value={playlistName}
          placeholder="A name for this collection"
        /></label
      ><label class="checkbox"
        ><input
          type="checkbox"
          bind:checked={remote}
          disabled={!data?.connected}
        />Create on YouTube as a private playlist</label
      >
      <p class="help">Local playlists are stored only on this device.</p>
      <div class="dialog-actions">
        <button type="button" onclick={() => (createOpen = false)}
          >Cancel</button
        ><button class="primary" disabled={busy}>Create playlist</button>
      </div>
    </form></Modal
  >{/if}
{#if adding}<Modal title="Add a track" close={() => (adding = false)}
    ><form
      onsubmit={async (event) => {
        event.preventDefault();
        const id = videoId(trackUrl);
        if (!id) {
          report('Enter a valid YouTube video URL or video ID.');
          return;
        }
        if (
          await run(
            () => call('add_track', { playlistId: addId, videoId: id }),
            'Track added',
          )
        )
          adding = false;
      }}
    >
      <label
        >Playlist<select bind:value={addId} required
          ><option value="" disabled>Choose a playlist</option
          >{#each data?.library.playlists ?? [] as playlist (playlist.id)}<option
              value={playlist.id}>{playlist.name}</option
            >{/each}</select
        ></label
      ><label
        >YouTube video URL<input
          bind:value={trackUrl}
          required
          placeholder="https://music.youtube.com/watch?v=…"
        /></label
      >
      <p class="help">
        Track title and artist are loaded from YouTube. Adding to a YouTube
        playlist changes your account.
      </p>
      <div class="dialog-actions">
        <button type="button" onclick={() => (adding = false)}>Cancel</button
        ><button class="primary" disabled={busy || !addId}>Add track</button>
      </div>
    </form></Modal
  >{/if}
{#if removing}<Modal title="Remove track?" close={() => (removing = null)}
    ><p>
      Remove “{removing.title}” from {selected?.name}? {selected?.remoteId
        ? 'This also removes the entry from YouTube.'
        : ''}
    </p>
    <p class="help">
      An enabled rule may add it again if it is still in a source playlist.
    </p>
    <div class="dialog-actions">
      <button onclick={() => (removing = null)}>Cancel</button><button
        class="danger"
        disabled={busy}
        onclick={async () => {
          if (
            await run(
              () =>
                call('remove_track', {
                  playlistId: selected?.id,
                  videoId: removing?.id,
                  itemId: removing?.itemId,
                }),
              'Track removed',
            )
          )
            removing = null;
        }}>Remove track</button
      >
    </div></Modal
  >{/if}
{#if deleting}<Modal title="Delete playlist?" close={() => (deleting = null)}
    ><p>
      Delete the local playlist “{deleting.name}”? Other playlists are
      unaffected.
    </p>
    <div class="dialog-actions">
      <button onclick={() => (deleting = null)}>Cancel</button><button
        class="danger"
        disabled={busy}
        onclick={async () => {
          if (
            await run(
              () => call('delete_playlist', { id: deleting?.id }),
              'Playlist deleted',
            )
          ) {
            deleting = null;
            page = 'library';
          }
        }}>Delete playlist</button
      >
    </div></Modal
  >{/if}
