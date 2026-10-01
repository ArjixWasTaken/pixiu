/**
 * Songs made available offline are kept by the service worker under their
 * stream's address (`/rest/stream?id=…`), less who asked for it: the same
 * song, format and bitrate, whichever session plays it.
 */

/** The Subsonic parameters that say who asks, not what. */
const AUTH_PARAMS = ['u', 't', 's', 'p', 'apiKey', 'c', 'v', 'f']

export const isStreamUrl = (url: URL) => /\/rest\/stream(\.view)?$/.test(url.pathname)

/** The key a stream is kept under. */
export const streamCacheKey = (url: string) => {
  const key = new URL(url)
  AUTH_PARAMS.forEach(name => key.searchParams.delete(name))
  key.searchParams.sort()
  return key.toString()
}

/** The song a stream plays. */
export const streamSongId = (url: string) => new URL(url, location.origin).searchParams.get('id')
