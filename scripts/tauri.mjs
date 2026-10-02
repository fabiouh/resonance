import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { loadEnvironment } from './environment.mjs';

loadEnvironment();
const require = createRequire(import.meta.url);
const child = spawn(
  process.execPath,
  [require.resolve('@tauri-apps/cli/tauri.js'), ...process.argv.slice(2)],
  {
    stdio: 'inherit',
    env: process.env,
  },
);
child.on('error', () => {
  console.error('Could not start the Tauri CLI. Run pnpm install first.');
  process.exitCode = 1;
});
child.on('exit', (code) => {
  process.exitCode = code ?? 1;
});
