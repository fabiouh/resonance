import { writeFileSync } from 'node:fs';

const key = process.env.TAURI_SIGNING_PUBLIC_KEY?.trim();
if (!key)
  throw new Error(
    'Set the TAURI_SIGNING_PUBLIC_KEY repository variable before releasing.',
  );
const decoded = Buffer.from(key, 'base64').toString('utf8');
if (!decoded.startsWith('untrusted comment:') || !decoded.includes('\n'))
  throw new Error('The updater public key is not a Tauri public key.');
const config = {
  bundle: { createUpdaterArtifacts: true },
  plugins: {
    updater: {
      pubkey: key,
      endpoints: [
        'https://github.com/fabiouh/resonance/releases/latest/download/latest.json',
      ],
    },
  },
};
writeFileSync(
  'src-tauri/tauri.updater.conf.json',
  `${JSON.stringify(config, null, 2)}\n`,
);
