<script lang="ts">
  import { Plus, GitMerge, Trash2, Pencil, Play } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { call } from '../lib/api';
  import type { Library, Rule, PlaylistDiff } from '../lib/types';
  let {
    library,
    run,
  }: {
    library: Library;
    run: (action: () => Promise<void>, success?: string) => Promise<boolean>;
  } = $props();
  let editor = $state<Rule | null>(null);
  let previews = $state<PlaylistDiff[] | null>(null);
  let deleting = $state<Rule | null>(null);
  const name = (id: string) =>
    library.playlists.find((p) => p.id === id)?.name ?? 'Missing playlist';
  function create() {
    editor = {
      id: '',
      name: 'Auto All',
      sources: [],
      target: '',
      enabled: true,
      managed: {},
    };
  }
  async function save() {
    if (await run(() => call('save_rule', { rule: editor }), 'Rule saved'))
      editor = null;
  }
  async function preview() {
    await run(async () => {
      await call('sync_library', { apply: false });
      previews = await call<PlaylistDiff[]>('preview_rules');
    });
  }
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">A LITTLE LESS UPKEEP</div>
    <h1>Curation rules</h1>
    <p>Bring source playlists together. Keep the tracks that belong.</p>
  </div>
  <button
    class="primary"
    onclick={create}
    disabled={library.playlists.length < 2}><Plus size={16} />New rule</button
  >
</div>
<div class="info-strip">
  <GitMerge size={20} />
  <p>
    Auto All collects unique tracks from selected playlists. It removes only
    tracks the rule added, and only after the last source removes them.
  </p>
</div>
{#if library.rules.length === 0}
  <div class="empty-state">
    <GitMerge size={38} />
    <h2>No curation rules yet</h2>
    <p>
      Create at least two playlists, then choose sources and a target for your
      first rule.
    </p>
  </div>
{:else}
  <div class="rule-list">
    {#each library.rules as rule (rule.id)}<article class="rule-row">
        <div>
          <div class="rule-title">
            <h2>{rule.name}</h2>
            <span class:enabled={rule.enabled} class="status-dot"></span><span
              class="muted">{rule.enabled ? 'Enabled' : 'Paused'}</span
            >
          </div>
          <p>
            {rule.sources.map(name).join(', ')} <span class="muted">→</span>
            {name(rule.target)}
          </p>
          <small>{Object.keys(rule.managed).length} managed tracks</small>
        </div>
        <div class="button-row">
          <button
            class="icon-button"
            aria-label={`Edit ${rule.name}`}
            onclick={() => (editor = structuredClone(rule))}
            ><Pencil size={16} /></button
          ><button
            class="icon-button"
            aria-label={`Delete ${rule.name}`}
            onclick={() => (deleting = rule)}><Trash2 size={16} /></button
          >
        </div>
      </article>{/each}
  </div>
  <button onclick={preview}
    ><Play size={16} />Refresh and preview changes</button
  >
{/if}
<p class="help section-footnote">
  Manual sync refreshes playlists without applying rules. Review changes here,
  or enable background synchronization in Settings to apply enabled rules
  automatically. Targets cannot feed other rules.
</p>

{#if editor}
  <Modal
    title={editor.id ? 'Edit curation rule' : 'New curation rule'}
    close={() => (editor = null)}
  >
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label
        >Name<input bind:value={editor.name} required maxlength="120" /></label
      >
      <label
        >Target playlist<select
          bind:value={editor.target}
          required
          disabled={!!editor.id}
          ><option value="" disabled>Choose a target</option
          >{#each library.playlists as playlist (playlist.id)}<option
              value={playlist.id}
              >{playlist.name}{playlist.remoteId
                ? ' · YouTube'
                : ' · Local'}</option
            >{/each}</select
        ></label
      >
      <fieldset>
        <legend>Source playlists</legend
        >{#each library.playlists.filter((p) => p.id !== editor?.target) as playlist (playlist.id)}<label
            class="checkbox"
            ><input
              type="checkbox"
              value={playlist.id}
              bind:group={editor.sources}
            />{playlist.name}</label
          >{/each}
      </fieldset>
      <label class="checkbox"
        ><input type="checkbox" bind:checked={editor.enabled} />Enable this rule</label
      >
      <p class="help">
        Existing target tracks remain yours. Disabling or deleting a rule leaves
        its tracks in place.
      </p>
      <div class="dialog-actions">
        <button type="button" onclick={() => (editor = null)}>Cancel</button
        ><button class="primary">Save rule</button>
      </div>
    </form>
  </Modal>
{/if}
{#if previews}
  <Modal title="Review curation changes" close={() => (previews = null)}>
    <div class="preview-list">
      {#each previews as preview (preview.ruleId)}<div>
          <h3>{library.rules.find((r) => r.id === preview.ruleId)?.name}</h3>
          <p>
            <span class="accent">+{preview.add.length} additions</span> · {preview
              .remove.length} removals
          </p>
          {#each preview.add as track (track.id)}<p class="preview-track">
              + {track.title}
            </p>{/each}{#each preview.remove as track (track.itemId ?? track.id)}<p
              class="preview-track danger-text"
            >
              − {track.title}
            </p>{/each}
        </div>{/each}
    </div>
    <p class="help">
      Applying refreshes YouTube again before calculating changes. Source
      changes since this preview will be included.
    </p>
    <div class="dialog-actions">
      <button onclick={() => (previews = null)}>Cancel</button><button
        class="primary"
        disabled={!previews.some((p) => p.add.length || p.remove.length)}
        onclick={async () => {
          if (
            await run(
              () => call('sync_library', { apply: true }),
              'Curation complete',
            )
          )
            previews = null;
        }}>Apply enabled rules</button
      >
    </div>
  </Modal>
{/if}
{#if deleting}<Modal title="Delete rule?" close={() => (deleting = null)}
    ><p>
      Tracks added by {deleting.name} will remain in the target playlist and will
      no longer be managed.
    </p>
    <div class="dialog-actions">
      <button onclick={() => (deleting = null)}>Cancel</button><button
        class="danger"
        onclick={async () => {
          if (
            await run(
              () => call('delete_rule', { id: deleting?.id }),
              'Rule deleted',
            )
          )
            deleting = null;
        }}>Delete rule</button
      >
    </div></Modal
  >{/if}
