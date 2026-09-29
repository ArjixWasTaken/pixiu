/**
 * píxiū's library lists: albums, artists and songs sorted by any column and
 * paged with a cursor, which Subsonic cannot do. Items arrive as Subsonic
 * JSON and leave as koel's types.
 */
import { http } from '@/services/http'
import { subsonic } from '@/services/subsonic'

export interface ListParams {
  sort?: MaybeArray<string>
  order?: SortOrder
  cursor?: string | null
  limit?: number
  favorites_only?: boolean
  genre?: string
  artist?: string
}

const query = (params: ListParams) => {
  const search = new URLSearchParams()

  for (const [name, value] of Object.entries(params)) {
    // Koel may sort by several fields; píxiū sorts by the first.
    const single = Array.isArray(value) ? value[0] : value

    if (single !== undefined && single !== null && single !== '') {
      search.set(name, String(single))
    }
  }

  return search
}

const list = async <T>(path: string, params: ListParams, map: (item: Record<string, any>) => T) => {
  const resource = await http.get<CursorPaginatorResource<Record<string, any>>>(`${path}?${query(params)}`)

  return {
    items: resource.data.map(map),
    nextCursor: resource.meta.next_cursor,
  }
}

export const library = {
  albums: (params: ListParams) => list('albums', params, subsonic.toAlbum),
  artists: (params: ListParams) => list('artists', params, subsonic.toArtist),
  songs: (params: ListParams) => list('songs', params, subsonic.toSong),

  async recentlyPlayed(limit: number) {
    return (await http.get<Record<string, any>[]>(`songs/recently-played?limit=${limit}`)).map(subsonic.toSong)
  },
}
