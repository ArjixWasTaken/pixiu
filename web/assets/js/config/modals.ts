export interface Modals {
  ABOUT_KOEL: never
  CREATE_PLAYLIST_FORM: { folder: PlaylistFolder | null; playables: Playable[] }
  CREATE_PLAYLIST_FOLDER_FORM: { parent: PlaylistFolder | null }
  CREATE_SMART_PLAYLIST_FORM: { folder: PlaylistFolder | null }
  EDIT_ALBUM_FORM: { album: Album }
  EDIT_ARTIST_FORM: { artist: Artist }
  EDIT_PLAYLIST_FORM: { playlist: Playlist }
  EDIT_PLAYLIST_FOLDER_FORM: { folder: PlaylistFolder }
  EDIT_SMART_PLAYLIST_FORM: { playlist: Playlist }
  EDIT_SONG_FORM: { songs: Song[]; initialTab: EditSongFormTabName }
  EQUALIZER: never
  SONG_INFO: { song: Song }
  REORDER_HOME_BLOCKS: { blocks: { id: string; label: string }[] }
}
