/// <reference lib="webworker" />

/**
 * píxiū's service worker (built by vite-plugin-pwa, on Workbox). It keeps:
 * - the player itself, precached at each build, so it opens without the server;
 * - what the player asks for at start-up, network first, so it starts with
 *   the last of it when the server can't be reached;
 * - the fonts and icons (from Google Fonts), so they show offline too;
 * - the songs made available offline, on the player's request (see
 *   useOfflinePlayback), answering the audio element from them, seeking
 *   (Range requests) included.
 */

import { cleanupOutdatedCaches, createHandlerBoundToURL, precacheAndRoute } from 'workbox-precaching'
import { NavigationRoute, registerRoute } from 'workbox-routing'
import { CacheFirst, NetworkFirst, StaleWhileRevalidate } from 'workbox-strategies'
import { createPartialResponse } from 'workbox-range-requests'
import { isStreamUrl, streamCacheKey } from '@/utils/streamCache'

declare const self: ServiceWorkerGlobalScope & {
  __WB_MANIFEST: Array<{ url: string; revision: string | null }>
}

const AUDIO_CACHE_NAME = 'pixiu-audio-v1'
const START_UP_CACHE_NAME = 'pixiu-start-up-v1'
const FONT_STYLES_CACHE_NAME = 'pixiu-font-styles-v1'
const FONTS_CACHE_NAME = 'pixiu-fonts-v1'

// ---- The player ----

// eslint-disable-next-line no-underscore-dangle -- Workbox's injection point, named so.
precacheAndRoute(self.__WB_MANIFEST)
cleanupOutdatedCaches()

// Every address the player routes itself opens the player; the APIs are the server's.
registerRoute(new NavigationRoute(createHandlerBoundToURL('index.html'), { denylist: [/^\/(api|rest)(\/|$)/] }))

// ---- Start-up ----

const isStartUp = (url: URL) => /^\/(api\/(bootstrap|playlists)|rest\/getPlayQueue(\.view)?)$/.test(url.pathname)

registerRoute(
  ({ url, request }) => request.method === 'GET' && url.origin === self.location.origin && isStartUp(url),
  new NetworkFirst({ cacheName: START_UP_CACHE_NAME, networkTimeoutSeconds: 5 }),
)

// ---- Fonts and icons ----

// The stylesheets are kept, and refreshed in the background.
registerRoute(
  ({ url }) => url.origin === 'https://fonts.googleapis.com',
  new StaleWhileRevalidate({ cacheName: FONT_STYLES_CACHE_NAME }),
)

// A font file never changes at its address (a new version comes at a new one), so a kept one is used as it is.
registerRoute(
  ({ url }) => url.origin === 'https://fonts.gstatic.com',
  new CacheFirst({
    cacheName: FONTS_CACHE_NAME,
    plugins: [
      {
        // Only the latest few: older versions are left behind as the stylesheets move on.
        cacheDidUpdate: async ({ cacheName }) => {
          const cache = await caches.open(cacheName)
          const kept = await cache.keys()
          await Promise.all(kept.slice(0, -30).map(request => cache.delete(request)))
        },
      },
    ],
  }),
)

// ---- Songs made available offline ----

registerRoute(
  ({ url }) => url.origin === self.location.origin && isStreamUrl(url),
  async ({ request }) => {
    const cached = await (await caches.open(AUDIO_CACHE_NAME)).match(streamCacheKey(request.url))

    if (!cached) {
      // Not kept: it streams from the server as usual.
      return fetch(request)
    }

    return request.headers.has('range') ? createPartialResponse(request, cached) : cached
  },
)

export interface CacheAudioMessage {
  type: 'CACHE_AUDIO'
  songId: string
  sourceUrl: string
}

export interface DeleteAudioCacheMessage {
  type: 'DELETE_AUDIO_CACHE'
  songId: string
  sourceUrl: string
}

export interface GetCacheStatusMessage {
  type: 'GET_CACHE_STATUS'
  sourceUrls: string[]
}

type Message = CacheAudioMessage | DeleteAudioCacheMessage | GetCacheStatusMessage | { type: 'SKIP_WAITING' }

/** Fetches a song whole and keeps it, telling the player how far along it is. */
const cacheAudio = async ({ songId, sourceUrl }: CacheAudioMessage, client: Client) => {
  const cache = await caches.open(AUDIO_CACHE_NAME)
  const key = streamCacheKey(sourceUrl)

  if (await cache.match(key)) {
    client.postMessage({ type: 'CACHE_AUDIO_COMPLETE', songId })
    return
  }

  try {
    const response = await fetch(sourceUrl)

    if (!response.ok || !response.body) {
      throw new Error(`HTTP ${response.status}`)
    }

    const type = response.headers.get('Content-Type') || 'audio/mpeg'
    const total = Number(response.headers.get('Content-Length') || 0)
    const reader = response.body.getReader()
    const chunks: BlobPart[] = []
    let received = 0

    while (true) {
      const { done, value } = await reader.read()

      if (done) {
        break
      }

      chunks.push(value)
      received += value.length

      if (total > 0) {
        client.postMessage({ type: 'CACHE_AUDIO_PROGRESS', songId, progress: received / total, received, total })
      }
    }

    const blob = new Blob(chunks, { type })
    await cache.put(
      key,
      new Response(blob, {
        headers: { 'Content-Type': type, 'Content-Length': String(blob.size), 'Accept-Ranges': 'bytes' },
      }),
    )

    client.postMessage({ type: 'CACHE_AUDIO_COMPLETE', songId })
  } catch (error) {
    client.postMessage({
      type: 'CACHE_AUDIO_ERROR',
      songId,
      error: error instanceof Error ? error.message : 'Unknown error',
    })
  }
}

const deleteAudioCache = async ({ songId, sourceUrl }: DeleteAudioCacheMessage, client: Client) => {
  const deleted = await (await caches.open(AUDIO_CACHE_NAME)).delete(streamCacheKey(sourceUrl))
  client.postMessage({ type: 'DELETE_AUDIO_CACHE_COMPLETE', songId, deleted })
}

const getCacheStatus = async ({ sourceUrls }: GetCacheStatusMessage, client: Client) => {
  const cache = await caches.open(AUDIO_CACHE_NAME)
  const statuses: Record<string, boolean> = {}

  for (const url of sourceUrls) {
    statuses[url] = Boolean(await cache.match(streamCacheKey(url)))
  }

  client.postMessage({ type: 'CACHE_STATUS', statuses })
}

self.addEventListener('message', (event: ExtendableMessageEvent) => {
  const data = event.data as Message
  const client = event.source as Client

  switch (data.type) {
    case 'CACHE_AUDIO':
      event.waitUntil(cacheAudio(data, client))
      break

    case 'DELETE_AUDIO_CACHE':
      event.waitUntil(deleteAudioCache(data, client))
      break

    case 'GET_CACHE_STATUS':
      event.waitUntil(getCacheStatus(data, client))
      break

    // A newer version waits until the user reloads into it (UpdateNotification).
    case 'SKIP_WAITING':
      self.skipWaiting()
      break
  }
})

// The first version takes the open player over at once: songs can be made available offline right away.
self.addEventListener('activate', (event: ExtendableEvent) => event.waitUntil(self.clients.claim()))
