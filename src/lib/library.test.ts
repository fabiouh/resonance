import { describe, expect, it } from 'vitest';
import { allTracks, filterTracks, videoId } from './library';
import type { Playlist, Track } from './types';

describe('YouTube links', () => {
  it('accepts music, short, embed and watch URLs', () => {
    for (const url of [
      'https://music.youtube.com/watch?v=abcdefghijk&list=PL1',
      'https://youtu.be/abcdefghijk?t=2',
      'https://www.youtube.com/shorts/abcdefghijk',
      'abcdefghijk',
    ]) {
      expect(videoId(url)).toBe('abcdefghijk');
    }
  });
  it('rejects unrelated hosts, malformed IDs and unsafe schemes', () => {
    for (const url of [
      'https://youtube.com.evil.test/watch?v=abcdefghijk',
      'https://example.com/abcdefghijk',
      'javascript:abcdefghijk',
      'https://youtube.com/watch?v=short',
      'https://youtu.be/abcdefghijk/more',
    ])
      expect(videoId(url)).toBeNull();
  });
});

it('deduplicates the library without changing playlist membership', () => {
  const track: Track = {
    id: 'abcdefghijk',
    title: 'Track',
    artist: 'Artist',
    itemId: null,
  };
  const playlists: Playlist[] = ['a', 'b'].map((id) => ({
    id,
    name: id,
    remoteId: null,
    tracks: [track],
  }));
  expect(allTracks(playlists)).toEqual([track]);
  expect(playlists.flatMap((p) => p.tracks)).toHaveLength(2);
  expect(filterTracks([track], ' ARTIST ')).toHaveLength(1);
  expect(filterTracks([track], 'missing')).toHaveLength(0);
});
