import { existsSync } from 'node:fs';

export function loadEnvironment(path = '.env') {
  if (!existsSync(path)) return;
  try {
    process.loadEnvFile(path);
  } catch {
    throw new Error(
      'Could not load the environment file. Check its format and permissions.',
    );
  }
}
