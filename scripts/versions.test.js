import { expect, it } from 'vitest';
import { nextVersion, versions } from './versions.mjs';

it('bumps semantic versions and resets lower components', () => {
  expect(nextVersion('0.9.3', 'patch')).toBe('0.9.4');
  expect(nextVersion('0.9.3', 'minor')).toBe('0.10.0');
  expect(nextVersion('0.9.3', 'major')).toBe('1.0.0');
  expect(() => nextVersion('0.1.0-beta', 'patch')).toThrow();
  expect(() => nextVersion('0.1.0', 'unknown')).toThrow();
});

it('keeps application manifests and Cargo lockfile in sync', () => {
  expect(versions()).toMatch(/^\d+\.\d+\.\d+$/);
});
