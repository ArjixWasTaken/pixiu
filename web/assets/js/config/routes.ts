import type { Route } from '@/router'
import { cache } from '@/services/cache'
import { usePolicies } from '@/composables/usePolicies'

// píxiū's ids, as its Subsonic API writes them.
const ALBUM_ID = 'al-[0-9]+'
const ARTIST_ID = 'ar-[0-9]+'
const PLAYLIST_ID = 'pl-[0-9]+'
const SONG_ID = 'tr-[0-9]+'
// The tokens of links píxiū emails, and of single sign-ons: base64url.
const EMAIL_TOKEN = '[A-Za-z0-9_-]+'

export const routes = [
  {
    name: 'home',
    path: '/home',
    screen: 'Home',
  },
  {
    name: '404',
    path: '/404',
    screen: '404',
    meta: {
      public: true,
    },
  },
  {
    name: 'queue',
    path: '/queue',
    screen: 'Queue',
  },
  {
    name: 'songs.index',
    path: '/songs',
    screen: 'Songs',
  },
  {
    name: 'albums.index',
    path: '/albums',
    screen: 'Albums',
  },
  {
    name: 'artists.index',
    path: '/artists',
    screen: 'Artists',
  },
  {
    name: 'favorites',
    path: '/favorites',
    screen: 'Favorites',
  },
  {
    name: 'recently-played',
    path: '/recently-played',
    screen: 'RecentlyPlayed',
  },
  {
    name: 'offline-songs',
    path: '/offline-songs',
    screen: 'OfflineSongs',
  },
  {
    name: 'search',
    path: '/search',
    screen: 'Search.Excerpt',
  },
  {
    name: 'search.playables',
    path: '/search/songs',
    screen: 'Search.Playables',
  },
  {
    name: 'upload',
    path: '/upload',
    screen: 'Upload',
    meta: {
      guard: () => usePolicies().currentUserCan.uploadSongs(),
    },
  },
  {
    name: 'hunt',
    path: '/discover',
    screen: 'Hunt',
  },
  {
    name: 'watches',
    path: '/watches',
    screen: 'Watches',
  },
  {
    name: 'jobs',
    path: '/jobs',
    screen: 'Jobs',
  },
  {
    name: 'orphans',
    path: '/orphans',
    screen: 'Orphans',
  },
  {
    name: 'settings',
    path: '/settings',
    screen: 'Settings',
  },
  {
    name: 'profile',
    path: '/profile',
    screen: 'Profile',
  },
  {
    name: 'albums.show',
    path: '/albums/:id/:tab?',
    screen: 'Album',
    constraints: {
      id: ALBUM_ID,
      tab: '(songs|other-albums|information)',
    },
  },
  {
    name: 'artists.show',
    path: '/artists/:id/:tab?',
    screen: 'Artist',
    constraints: {
      id: ARTIST_ID,
      tab: '(songs|albums|information)',
    },
  },
  {
    name: 'playlists.show',
    path: '/playlists/:id',
    screen: 'Playlist',
    constraints: {
      id: PLAYLIST_ID,
    },
  },
  {
    name: 'genres.index',
    path: '/genres',
    screen: 'Genres',
  },
  {
    name: 'genres.show',
    path: '/genres/:id',
    screen: 'Genre',
  },
  {
    name: 'songs.queue',
    path: '/songs/:id',
    screen: 'Queue',
    constraints: {
      id: SONG_ID,
    },
    meta: {
      redirect: () => 'queue',
      onResolved: params => cache.set('playable-to-queue', params.id),
    },
  },
  {
    name: 'verify-email',
    path: '/verify-email/:token',
    screen: 'VerifyEmail',
    meta: {
      public: true,
      layout: 'email-link',
    },
    constraints: {
      token: EMAIL_TOKEN,
    },
  },
  {
    name: 'reset-password',
    path: '/reset-password/:token',
    screen: 'ResetPassword',
    meta: {
      public: true,
      layout: 'email-link',
    },
    constraints: {
      token: EMAIL_TOKEN,
    },
  },
  {
    name: 'sso',
    path: '/sso/:code',
    screen: 'SsoComplete',
    meta: {
      public: true,
      layout: 'sso',
    },
    constraints: {
      code: EMAIL_TOKEN,
    },
  },
] as const satisfies Route[]

export type RouteName = (typeof routes)[number]['name'] | keyof RouteNames
