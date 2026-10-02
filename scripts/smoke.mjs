import { spawn } from 'node:child_process';
import { mkdtemp, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { chromium, expect } from '@playwright/test';
import { setTimeout as delay } from 'node:timers/promises';
import { createServer } from 'node:net';

if (process.platform !== 'win32')
  throw new Error('Desktop smoke tests require Windows.');
const release = process.argv.includes('--release');
const binaryArgument = process.argv.indexOf('--binary');
if (binaryArgument >= 0 && !process.argv[binaryArgument + 1])
  throw new Error('--binary requires an executable path.');
const profile = await mkdtemp(join(tmpdir(), 'resonance-smoke-'));
const server = createServer();
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
await new Promise((resolve) => server.close(resolve));
const binary = resolve(
  binaryArgument >= 0
    ? process.argv[binaryArgument + 1]
    : `src-tauri/target/${release ? 'release' : 'debug'}/resonance.exe`,
);
const child = spawn(binary, [], {
  windowsHide: true,
  env: {
    ...process.env,
    RESONANCE_TEST_DATA_DIR: profile,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
  stdio: 'ignore',
});
let launchError;
const stopped = new Promise((resolve) => child.once('close', resolve));
child.on('error', (error) => {
  launchError = error;
});
let browser;
try {
  const endpoint = `http://127.0.0.1:${port}`;
  for (let attempt = 0; attempt < 60; attempt++) {
    if (launchError) throw launchError;
    if (child.exitCode !== null)
      throw new Error(`Application exited during startup (${child.exitCode}).`);
    try {
      if ((await fetch(`${endpoint}/json/version`)).ok) break;
    } catch {
      /* WebView2 starts asynchronously. */
    }
    await delay(500);
  }
  browser = await chromium.connectOverCDP(endpoint);
  const context = browser.contexts()[0];
  let page;
  for (let attempt = 0; attempt < 40; attempt++) {
    page = context
      .pages()
      .find((page) => page.url().includes('tauri.localhost'));
    if (page) break;
    await delay(250);
  }
  if (!page) throw new Error('The native application window did not load.');
  await expect(
    page.getByRole('heading', { name: 'Your library', exact: true }),
  ).toBeVisible();
  if (!release) {
    await page
      .getByRole('button', { name: 'New playlist', exact: true })
      .click();
    await page.getByLabel('Name', { exact: true }).fill('Evening rotation');
    await page
      .getByRole('dialog')
      .getByRole('button', { name: 'Create playlist', exact: true })
      .click();
    await expect(
      page.getByRole('status').filter({ hasText: 'Playlist created' }),
    ).toBeVisible();
    await page
      .getByRole('navigation', { name: 'Playlists', exact: true })
      .getByRole('button', { name: 'Evening rotation' })
      .click();
    await expect(
      page.getByRole('heading', { name: 'Evening rotation' }),
    ).toBeVisible();
    await page
      .getByRole('button', { name: 'Rename playlist', exact: true })
      .click();
    await page
      .getByRole('dialog')
      .getByLabel('Name', { exact: true })
      .fill('Evening collection');
    await page.getByRole('button', { name: 'Save name', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'Evening collection' }),
    ).toBeVisible();
    await page
      .getByRole('button', { name: 'Listening history', exact: true })
      .click();
    await page.getByRole('button', { name: /Google history/ }).click();
    const historyFile = {
      name: 'watch-history.json',
      mimeType: 'application/json',
      buffer: Buffer.from(
        JSON.stringify([
          {
            title: 'Watched Imported example',
            titleUrl: 'https://www.youtube.com/watch?v=aqz-KE-bpKQ',
            time: '2024-01-01T12:00:00.123Z',
          },
        ]),
      ),
    };
    await page
      .getByLabel('Google Takeout history file')
      .setInputFiles(historyFile);
    await expect(
      page.getByRole('status').filter({ hasText: '1 events imported' }),
    ).toBeVisible();
    await expect(
      page.getByText('Imported example', { exact: true }),
    ).toBeVisible();
    await page
      .getByLabel('Google Takeout history file')
      .setInputFiles(historyFile);
    await expect(
      page
        .getByRole('status')
        .filter({ hasText: '0 events imported; 1 duplicates' }),
    ).toBeVisible();
    await page
      .getByRole('navigation', { name: 'Playlists', exact: true })
      .getByRole('button', { name: 'Evening collection' })
      .click();
    if (process.argv.includes('--online')) {
      await page
        .getByRole('button', { name: 'Add track', exact: true })
        .click();
      await page
        .getByLabel('YouTube video URL')
        .fill('https://www.youtube.com/watch?v=aqz-KE-bpKQ');
      await page
        .getByRole('dialog')
        .getByRole('button', { name: 'Add track', exact: true })
        .click();
      await expect(
        page.getByRole('status').filter({ hasText: 'Track added' }),
      ).toBeVisible({ timeout: 40000 });
      await page.getByRole('button', { name: /^Play Big Buck Bunny/ }).click();
      await expect
        .poll(
          async () =>
            Number(
              await page
                .getByRole('slider', { name: 'Playback position' })
                .inputValue(),
            ),
          { timeout: 45000 },
        )
        .toBeGreaterThan(2);
      await delay(12000);
      await page
        .getByRole('button', { name: 'Pause (Space)', exact: true })
        .click();
      await page
        .getByRole('button', { name: 'Listening history', exact: true })
        .click();
      await expect(
        page.getByText('unique tracks', { exact: true }),
      ).toBeVisible();
      console.log('YouTube metadata, playback, and listening history passed.');
    }
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByLabel('Background interval').selectOption('30');
    await page
      .getByRole('button', { name: 'Save settings', exact: true })
      .click();
    await expect(
      page.getByRole('status').filter({ hasText: 'Settings saved' }),
    ).toBeVisible();
    await page.reload();
    await expect(
      page.getByRole('heading', { name: 'Your library', exact: true }),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await expect(page.getByLabel('Background interval')).toHaveValue('30');
    await page
      .getByRole('button', { name: 'Listening history', exact: true })
      .click();
    await page
      .getByRole('button', { name: 'Google history (1)', exact: true })
      .click();
    await expect(
      page.getByText('Imported example', { exact: true }),
    ).toBeVisible();
    await page
      .getByRole('button', { name: 'Clear history', exact: true })
      .click();
    await page
      .getByRole('dialog')
      .getByRole('button', { name: 'Clear history', exact: true })
      .click();
    await expect(
      page.getByRole('button', { name: 'Google history (0)', exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText('No imported watch events yet.', { exact: true }),
    ).toBeVisible();
    await page
      .getByRole('navigation', { name: 'Playlists', exact: true })
      .getByRole('button', { name: 'Evening collection' })
      .click();
    await page
      .getByRole('button', { name: 'Delete playlist', exact: true })
      .click();
    await page
      .getByRole('dialog')
      .getByRole('button', { name: 'Delete playlist', exact: true })
      .click();
    await expect(
      page.getByRole('heading', { name: 'Your library', exact: true }),
    ).toBeVisible();
    await expect(
      page
        .getByRole('navigation', { name: 'Playlists', exact: true })
        .getByRole('button'),
    ).toHaveCount(0);
    await page
      .getByRole('button', { name: 'Library', exact: false })
      .first()
      .click();
  }
  await mkdir('artifacts', { recursive: true });
  await page.screenshot({
    path: `artifacts/${release ? 'release' : 'desktop'}-smoke.png`,
  });
  console.log(
    `Desktop ${release ? 'release startup' : 'library and settings'} smoke test passed.`,
  );
} finally {
  await browser?.close();
  if (child.exitCode === null) child.kill();
  await stopped;
}
