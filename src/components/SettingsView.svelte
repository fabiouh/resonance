<script lang="ts">
  import { untrack } from 'svelte';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { call, errorMessage } from '../lib/api';
  import type { Settings } from '../lib/types';
  let {
    settings,
    connected,
    updaterConfigured,
    run,
    notify,
  }: {
    settings: Settings;
    connected: boolean;
    updaterConfigured: boolean;
    run: (action: () => Promise<void>, success?: string) => Promise<boolean>;
    notify: (message: string) => void;
  } = $props();
  let form = $state<Settings>(untrack(() => ({ ...settings })));
  let clientSecret = $state('');
  let update = $state<Update | null>(null);
  let updateStatus = $state('');
  let checking = $state(false);

  async function save() {
    await run(
      () => call('save_settings', { settings: form }),
      'Settings saved',
    );
  }
  async function signIn() {
    const secret = clientSecret;
    clientSecret = '';
    await run(async () => {
      await call('save_settings', { settings: form });
      await call('sign_in', { clientSecret: secret });
    }, 'Google connected. Sync your library to import playlists.');
  }
  async function checkUpdate() {
    checking = true;
    try {
      await update?.close();
      update = await check();
      updateStatus = update
        ? `Version ${update.version} is available.`
        : 'You are using the latest release.';
    } catch (error) {
      notify(errorMessage(error));
    } finally {
      checking = false;
    }
  }
  async function installUpdate() {
    if (!update) return;
    checking = true;
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (error) {
      notify(errorMessage(error));
    } finally {
      checking = false;
    }
  }
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">MAKE IT YOURS</div>
    <h1>Settings</h1>
    <p>Connections, background behavior, and privacy.</p>
  </div>
</div>
<section class="settings-section">
  <div>
    <h2>Google account</h2>
    <p>
      Sync playlists owned by your YouTube account. Google sign-in opens in your
      default browser.
    </p>
  </div>
  <div class="settings-fields">
    <p class="connection-status">
      {connected ? 'Google connected' : 'Not connected'}
    </p>
    <label
      >Desktop OAuth client ID<input
        bind:value={form.clientId}
        disabled={connected}
        placeholder="Client ID from Google Cloud"
        autocomplete="off"
      /></label
    >
    {#if !connected}<label
        >Desktop OAuth client secret<input
          type="password"
          bind:value={clientSecret}
          placeholder="Stored in Windows Credential Manager"
          autocomplete="off"
        /></label
      >{/if}
    <p class="help">
      Enable YouTube Data API v3 in Google Cloud and create a Desktop OAuth
      client. Add your account as a test user if the consent screen is in
      testing. No credentials are bundled with Resonance.
    </p>
    {#if connected}<button
        onclick={() =>
          run(
            () => call('disconnect'),
            'Google disconnected. Cached playlists are now local and rules are paused.',
          )}>Disconnect Google</button
      >{:else}<button class="primary" onclick={signIn}
        >Sign in with Google</button
      >{/if}
  </div>
</section>
<section class="settings-section">
  <div>
    <h2>Synchronization</h2>
    <p>
      Background sync refreshes your library and applies enabled curation rules
      while Resonance is running.
    </p>
  </div>
  <div class="settings-fields">
    <label
      >Background interval<select bind:value={form.syncMinutes}
        ><option value={0}>Off — sync manually</option><option value={15}
          >Every 15 minutes</option
        ><option value={30}>Every 30 minutes</option><option value={60}
          >Every hour</option
        ><option value={360}>Every 6 hours</option></select
      ></label
    ><label class="checkbox"
      ><input type="checkbox" bind:checked={form.closeToTray} />Keep running in
      the tray when closed</label
    >
    <p class="help">
      YouTube applies daily API quotas. Longer intervals use less quota.
    </p>
  </div>
</section>
<section class="settings-section">
  <div>
    <h2>Discord Rich Presence</h2>
    <p>
      Share tracks playing in Resonance with Discord. Pausing playback clears
      your activity.
    </p>
  </div>
  <div class="settings-fields">
    <label class="checkbox"
      ><input type="checkbox" bind:checked={form.discordEnabled} />Share
      listening activity</label
    ><label
      >Discord application ID<input
        bind:value={form.discordApplicationId}
        placeholder="Application ID from Discord Developer Portal"
        inputmode="numeric"
      /></label
    >
    <p class="help">
      Discord must be running locally. Resonance reconnects automatically after
      Discord restarts.
    </p>
  </div>
</section>
<div class="settings-save">
  <button class="primary" onclick={save}>Save settings</button>
</div>
<section class="settings-section">
  <div>
    <h2>Application updates</h2>
    <p>
      Updates are verified using the release signing key before installation.
    </p>
  </div>
  <div class="settings-fields">
    {#if updaterConfigured}<button disabled={checking} onclick={checkUpdate}
        >{checking ? 'Working…' : 'Check for updates'}</button
      >{#if updateStatus}<p>{updateStatus}</p>{/if}{#if update}<button
          class="primary"
          disabled={checking}
          onclick={installUpdate}>Install and restart</button
        >{/if}{:else}<p class="help">
        Automatic updates are unavailable in this build. Download new installers
        from the repository's Releases page.
      </p>{/if}
  </div>
</section>
<section class="settings-section">
  <div>
    <h2>Your data</h2>
    <p>
      Playlists and listening history stay in your local SQLite database. Tokens
      stay in Windows Credential Manager. Resonance has no telemetry.
    </p>
  </div>
  <div class="settings-fields">
    <p class="help">
      YouTube receives player requests and account operations. Discord receives
      your current track only when sharing is enabled. Disconnecting removes
      stored Google credentials; revoke access in your Google account to
      invalidate existing grants.
    </p>
  </div>
</section>
