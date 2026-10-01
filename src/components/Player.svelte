<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Volume2,
    ExternalLink,
    Music2,
  } from '@lucide/svelte';
  import { call, errorMessage } from '../lib/api';
  import type { Track } from '../lib/types';
  let {
    track,
    next,
    previous,
    canNext,
    canPrevious,
    report,
  }: {
    track: Track | null;
    next: () => void;
    previous: () => void;
    canNext: boolean;
    canPrevious: boolean;
    report: (message: string) => void;
  } = $props();
  let player: YT.Player | undefined;
  let ready = $state(false);
  let playing = $state(false);
  let volume = $state(70);
  let elapsed = $state(0);
  let length = $state(0);
  let connected = $state(false);
  let container: HTMLDivElement;
  let loadedId = '';
  const time = (value: number) =>
    `${Math.floor(value / 60)}:${String(Math.floor(value % 60)).padStart(2, '0')}`;

  async function heartbeat() {
    try {
      connected = await call<boolean>('playback_tick', {
        track: playing ? track : null,
      });
    } catch (error) {
      report(errorMessage(error));
    }
  }

  function toggle() {
    if (playing) player?.pauseVideo();
    else player?.playVideo();
  }

  onMount(() => {
    let disposed = false;
    let previousHeartbeat = 0;
    const create = () => {
      if (disposed) return;
      player = new window.YT.Player(container, {
        width: 320,
        height: 200,
        playerVars: { playsinline: 1, origin: window.location.origin },
        events: {
          onReady: () => {
            ready = true;
            player?.setVolume(volume);
          },
          onStateChange: (event) => {
            playing = event.data === window.YT.PlayerState.PLAYING;
            void heartbeat();
            if (event.data === window.YT.PlayerState.ENDED) next();
          },
          onError: (event) => {
            playing = false;
            void heartbeat();
            report(
              Number(event.data) === 153
                ? 'YouTube could not identify the embedded player. Open this track in YouTube Music.'
                : 'YouTube cannot play this track here. It may be private, restricted, or unavailable for embedding.',
            );
          },
        },
      });
    };
    if (window.YT?.Player) create();
    else {
      window.onYouTubeIframeAPIReady = create;
      const script = document.createElement('script');
      script.src = 'https://www.youtube.com/iframe_api';
      script.onerror = () =>
        report('Could not load the YouTube player. Check your connection.');
      document.head.append(script);
    }
    const timer = setInterval(() => {
      if (!ready) return;
      elapsed = player?.getCurrentTime() ?? 0;
      length = player?.getDuration() ?? 0;
      if (Date.now() - previousHeartbeat >= 10_000) {
        previousHeartbeat = Date.now();
        void heartbeat();
      }
    }, 1000);
    const shortcut = (event: KeyboardEvent) => {
      if (
        event.target instanceof HTMLElement &&
        (event.target.closest('input, textarea, select, button, dialog') ||
          event.target.isContentEditable)
      )
        return;
      if (event.code === 'Space') {
        event.preventDefault();
        toggle();
      }
      if (event.altKey && event.code === 'ArrowRight') {
        event.preventDefault();
        next();
      }
      if (event.altKey && event.code === 'ArrowLeft') {
        event.preventDefault();
        previous();
      }
    };
    window.addEventListener('keydown', shortcut);
    return () => {
      disposed = true;
      clearInterval(timer);
      player?.destroy();
      window.removeEventListener('keydown', shortcut);
      void call('playback_tick', { track: null }).catch(() => {});
    };
  });

  $effect(() => {
    if (ready && track && track.id !== loadedId) {
      loadedId = track.id;
      playing = false;
      void call('playback_tick', { track: null });
      player?.loadVideoById(track.id);
    }
  });
</script>

<aside class="now-playing" aria-label="Now playing">
  <div class="section-label">NOW PLAYING</div>
  <div class="video-frame"><div bind:this={container}></div></div>
  {#if track}
    <h2>{track.title}</h2>
    <p>{track.artist}</p>
    <button
      class="text-button"
      onclick={() =>
        call('open_youtube', { videoId: track.id }).catch((error) =>
          report(errorMessage(error)),
        )}><ExternalLink size={14} />YouTube Music</button
    >
  {:else}
    <div class="player-empty">
      <Music2 size={30} />
      <h2>Room for a good track</h2>
      <p>Select a track from your library to start listening.</p>
    </div>
  {/if}
  <div class="player-note">
    Playback is provided by YouTube. Some videos cannot be embedded.
  </div>
  {#if connected}<p class="presence-status">Sharing on Discord</p>{/if}
</aside>

<footer class="player-bar">
  <div class="playing-title">
    <Music2 size={20} />
    <div>
      <strong>{track?.title ?? 'Nothing playing'}</strong><span
        >{track?.artist ?? 'Your library, at your pace.'}</span
      >
    </div>
  </div>
  <div class="transport">
    <div class="transport-buttons">
      <button
        class="icon-button"
        aria-label="Previous track (Alt+Left)"
        disabled={!canPrevious}
        onclick={previous}><SkipBack size={18} /></button
      ><button
        class="play-button"
        aria-label={playing ? 'Pause (Space)' : 'Play (Space)'}
        disabled={!track || !ready}
        onclick={toggle}
        >{#if playing}<Pause size={19} />{:else}<Play size={19} />{/if}</button
      ><button
        class="icon-button"
        aria-label="Next track (Alt+Right)"
        disabled={!canNext}
        onclick={next}><SkipForward size={18} /></button
      >
    </div>
    <div class="seek">
      <span>{time(elapsed)}</span><input
        aria-label="Playback position"
        type="range"
        min="0"
        max={length || 1}
        value={elapsed}
        disabled={!track}
        oninput={(event) =>
          player?.seekTo(Number(event.currentTarget.value), true)}
      /><span>{time(length)}</span>
    </div>
  </div>
  <label class="volume"
    ><Volume2 size={17} /><input
      aria-label="Volume"
      type="range"
      min="0"
      max="100"
      bind:value={volume}
      oninput={(event) => player?.setVolume(Number(event.currentTarget.value))}
    /></label
  >
</footer>
