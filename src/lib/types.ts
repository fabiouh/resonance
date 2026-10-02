export interface Track {
  id: string;
  title: string;
  artist: string;
  itemId: string | null;
}

export interface Playlist {
  id: string;
  name: string;
  remoteId: string | null;
  tracks: Track[];
}

export interface Rule {
  id: string;
  name: string;
  sources: string[];
  target: string;
  enabled: boolean;
  managed: Record<string, string>;
}

export interface Settings {
  clientId: string;
  discordApplicationId: string;
  discordEnabled: boolean;
  syncMinutes: number;
  closeToTray: boolean;
}

export interface Listening {
  track: Track;
  seconds: number;
  lastPlayed: number;
}

export interface Library {
  playlists: Playlist[];
  rules: Rule[];
  settings: Settings;
  listening: Record<string, Listening>;
  lastSync: number | null;
}

export interface Snapshot {
  library: Library;
  connected: boolean;
  syncError: string | null;
  updaterConfigured: boolean;
  googleSecretConfigured: boolean;
}

export interface PlaylistDiff {
  ruleId: string;
  add: Track[];
  remove: Track[];
}
