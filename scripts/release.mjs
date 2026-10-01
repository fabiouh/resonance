import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { nextVersion, versions } from './versions.mjs';

function git(...args) {
  return execFileSync('git', args, { encoding: 'utf8' }).trim();
}
function pnpm(...args) {
  const result = spawnSync(
    process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm',
    args,
    { stdio: 'inherit', shell: process.platform === 'win32' },
  );
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`pnpm ${args.join(' ')} failed. No release was published.`);
}
function clean() {
  if (git('status', '--porcelain'))
    throw new Error('Commit or stash working changes before releasing.');
}

if (process.platform !== 'win32')
  throw new Error('Run releases on Windows to validate the installer.');
const bump = process.argv[2];
const version = nextVersion(versions(), bump);
clean();
if (git('branch', '--show-current') !== 'main')
  throw new Error('Release from main.');
git('fetch', 'origin', 'main', '--tags');
if (git('rev-parse', 'HEAD') !== git('rev-parse', 'origin/main'))
  throw new Error(
    'Push validated main and synchronize with origin before releasing.',
  );
if (git('tag', '--list', `v${version}`))
  throw new Error(`Tag v${version} already exists.`);
pnpm('validate');
clean();
for (const path of ['package.json', 'src-tauri/tauri.conf.json']) {
  const value = JSON.parse(readFileSync(path, 'utf8'));
  value.version = version;
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}
writeFileSync(
  'src-tauri/Cargo.toml',
  readFileSync('src-tauri/Cargo.toml', 'utf8').replace(
    /^version = "[^"]+"/m,
    `version = "${version}"`,
  ),
);
writeFileSync(
  'src-tauri/Cargo.lock',
  readFileSync('src-tauri/Cargo.lock', 'utf8').replace(
    /(name = "resonance"\r?\n)version = "[^"]+"/,
    `$1version = "${version}"`,
  ),
);
pnpm(
  'exec',
  'prettier',
  '--write',
  'package.json',
  'src-tauri/tauri.conf.json',
);
pnpm('validate');
pnpm('tauri', 'build');
pnpm('test:desktop:release');
const paths = [
  'package.json',
  'src-tauri/tauri.conf.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
];
const changed = git('diff', '--name-only').split('\n');
if (
  changed.some((path) => !paths.includes(path)) ||
  git('ls-files', '--others', '--exclude-standard')
)
  throw new Error(
    'Unexpected changes appeared during validation. Review them before committing.',
  );
git('add', '--', ...paths);
git('diff', '--cached', '--check');
const diff = git('diff', '--cached', '--unified=0');
const additions = diff
  .split('\n')
  .filter((line) => line.startsWith('+') && !line.startsWith('+++'));
if (
  additions.some(
    (line) =>
      !/^\+\s*(?:"version": "\d+\.\d+\.\d+",?|version = "\d+\.\d+\.\d+")\s*$/.test(
        line,
      ),
  )
)
  throw new Error(
    'Staged changes contain more than version fields. Review before committing.',
  );
git('commit', '-m', `chore(release): bump version to ${version}`);
clean();
git('tag', '-a', `v${version}`, '-m', `Resonance v${version}`);
git('push', '--atomic', 'origin', 'HEAD:main', `refs/tags/v${version}`);
console.log(
  `Published tag v${version}. GitHub Actions will build and publish the installer.`,
);
