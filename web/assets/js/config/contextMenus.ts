export interface ContextMenus {
  ALBUM: { album: Album }
  ARTIST: { artist: Artist }
  GENRE: { genre: Genre }
  PLAYABLES: { playables: Playable[]; fromQueue?: boolean }
  PLAYLIST: { playlist: Playlist }
  PLAYLIST_FOLDER: { folder: PlaylistFolder }
}
