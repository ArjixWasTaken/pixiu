export const albumTableColumnConfig = {
  storageKey: 'album-table-columns',
  validColumns: ['name', 'artist', 'time', 'year', 'rating', 'favorite'] as const,
  defaultColumns: ['name', 'artist', 'year', 'rating', 'favorite'] as const,
  alwaysVisible: ['name'] as const,
} satisfies {
  storageKey: string
  validColumns: readonly AlbumTableColumnName[]
  defaultColumns: readonly AlbumTableColumnName[]
  alwaysVisible: readonly AlbumTableColumnName[]
}

export const artistTableColumnConfig = {
  storageKey: 'artist-table-columns',
  validColumns: ['name', 'rating', 'favorite'] as const,
  defaultColumns: ['name', 'rating', 'favorite'] as const,
  alwaysVisible: ['name'] as const,
} satisfies {
  storageKey: string
  validColumns: readonly ArtistTableColumnName[]
  defaultColumns: readonly ArtistTableColumnName[]
  alwaysVisible: readonly ArtistTableColumnName[]
}

export const playableListColumnConfig = {
  storageKey: 'playable-list-columns',
  validColumns: ['track', 'genre', 'year', 'title', 'artist', 'album', 'duration', 'play_count', 'rating'] as const,
  defaultColumns: ['track', 'title', 'artist', 'album', 'duration'] as const,
  alwaysVisible: ['title'] as const,
  responsive: true,
} satisfies {
  storageKey: string
  validColumns: readonly PlayableListColumnName[]
  defaultColumns: readonly PlayableListColumnName[]
  alwaysVisible: readonly PlayableListColumnName[]
  responsive: boolean
}
