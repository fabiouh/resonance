import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Snapshot } from './types';

export const desktop = isTauri();

export async function call<T = void>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!desktop)
    throw new Error(
      'Open the Resonance desktop application to use your library.',
    );
  return invoke<T>(command, args);
}

export function snapshot(): Promise<Snapshot> {
  return call('snapshot');
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
