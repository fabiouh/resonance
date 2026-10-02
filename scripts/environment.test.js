import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';

it('loads local defaults without overriding the calling environment', () => {
  const directory = mkdtempSync(join(tmpdir(), 'resonance-env-'));
  const file = join(directory, '.env');
  writeFileSync(
    file,
    'RESONANCE_ENV_TEST_VALUE=local\nRESONANCE_ENV_TEST_DEFAULT=default\n',
  );
  const module = new URL('./environment.mjs', import.meta.url).href;
  const code = `import { loadEnvironment } from ${JSON.stringify(module)};
    loadEnvironment(process.argv[1]);
    if (process.env.RESONANCE_ENV_TEST_VALUE !== 'process' || process.env.RESONANCE_ENV_TEST_DEFAULT !== 'default') process.exit(1);`;
  expect(() =>
    execFileSync(process.execPath, ['--input-type=module', '-e', code, file], {
      env: { ...process.env, RESONANCE_ENV_TEST_VALUE: 'process' },
      stdio: 'pipe',
    }),
  ).not.toThrow();
});
