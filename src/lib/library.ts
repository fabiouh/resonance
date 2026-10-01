import type { Playlist, Track } from './types';

export function videoId(input: string): string | null {
  const value = input.trim();
  const valid = (id: string | null) =>
    id && /^[\w-]{11}$/.test(id) ? id : null;
  if (valid(value)) return value;
  try {
    const url = new URL(value);
    if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
    if (url.hostname === 'youtu.be') return valid(url.pathname.slice(1));
    if (
      ![
        'youtube.com',
        'www.youtube.com',
        'music.youtube.com',
        'm.youtube.com',
      ].includes(url.hostname)
    )
      return null;
    if (url.pathname === '/watch') return valid(url.searchParams.get('v'));
    if (/^\/(shorts|embed)\//.test(url.pathname))
      return valid(url.pathname.split('/')[2]);
  } catch {
    return null;
  }
  return null;
}

export function allTracks(playlists: Playlist[]): Track[] {
  return [
    ...new Map(
      playlists.flatMap((p) => p.tracks).map((t) => [t.id, t]),
    ).values(),
  ];
}

export function filterTracks(tracks: Track[], query: string): Track[] {
  const term = query.trim().toLocaleLowerCase();
  return tracks.filter((track) =>
    `${track.title} ${track.artist}`.toLocaleLowerCase().includes(term),
  );
}

export function duration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return minutes >= 60
    ? `${Math.floor(minutes / 60)}h ${minutes % 60}m`
    : `${minutes}m ${Math.floor(seconds % 60)}s`;
}
