import type { RouteRecordRaw } from 'vue-router'
import { usePolicies } from '@/composables/usePolicies'

// Not lazy: they must open offline.
import QueueScreen from '@/components/screens/QueueScreen.vue'
import OfflineSongsScreen from '@/components/screens/OfflineSongsScreen.vue'

declare module 'vue-router' {
  interface RouteMeta {
    /** The screen the route shows. */
    screen?: ScreenName
    /** Shown without the app's shell: the sign-in page's place, for email links and single sign-on. */
    layout?: 'email-link' | 'sso'
    /** Open to anyone, signed in or not. */
    public?: boolean
    /** Whether the signed-in user may see the screen; the 404 screen shows otherwise. */
    guard?: () => boolean
  }
}

// píxiū's ids, as its Subsonic API writes them.
const ALBUM_ID = 'al-[0-9]+'
const ARTIST_ID = 'ar-[0-9]+'
const PLAYLIST_ID = 'pl-[0-9]+'
const SONG_ID = 'tr-[0-9]+'
// The tokens of links píxiū emails, and of single sign-ons: base64url.
const EMAIL_TOKEN = '[A-Za-z0-9_-]+'

/** The settings tabs as `?tab=` named them, before tabs lived in the hash. */
const LEGACY_SETTINGS_TABS: Record<string, string> = {
  users: 'admin-users',
  'sign-in': 'admin-sign-in',
  email: 'admin-email',
}

/** Email links and single sign-on render outside the shell (App.vue); the route only marks them. */
const OutsideTheShell = { render: () => null }

export const routes = [
  {
    // The root opens Home, keeping what the server sent along (`?sso_error=…`).
    path: '/',
    redirect: to => ({ path: '/home', query: to.query, hash: to.hash }),
  },
  {
    name: 'home',
    path: '/home',
    component: () => import('@/components/screens/HomeScreen.vue'),
    meta: { screen: 'Home' },
  },
  {
    name: '404',
    path: '/404',
    component: () => import('@/components/screens/NotFoundScreen.vue'),
    meta: { screen: '404', public: true },
  },
  {
    name: 'queue',
    path: '/queue',
    component: QueueScreen,
    meta: { screen: 'Queue' },
  },
  {
    name: 'songs.index',
    path: '/songs',
    component: () => import('@/components/screens/AllSongsScreen.vue'),
    meta: { screen: 'Songs' },
  },
  {
    name: 'albums.index',
    path: '/albums',
    component: () => import('@/components/screens/AlbumListScreen.vue'),
    meta: { screen: 'Albums' },
  },
  {
    name: 'artists.index',
    path: '/artists',
    component: () => import('@/components/screens/ArtistListScreen.vue'),
    meta: { screen: 'Artists' },
  },
  {
    name: 'favorites',
    path: '/favorites',
    component: () => import('@/components/screens/FavoritesScreen.vue'),
    meta: { screen: 'Favorites' },
  },
  {
    name: 'recently-played',
    path: '/recently-played',
    component: () => import('@/components/screens/RecentlyPlayedScreen.vue'),
    meta: { screen: 'RecentlyPlayed' },
  },
  {
    name: 'offline-songs',
    path: '/offline-songs',
    component: OfflineSongsScreen,
    meta: { screen: 'OfflineSongs' },
  },
  {
    name: 'search',
    path: '/search',
    component: () => import('@/components/screens/search/SearchExcerptsScreen.vue'),
    meta: { screen: 'Search.Excerpt' },
  },
  {
    name: 'search.playables',
    path: '/search/songs',
    component: () => import('@/components/screens/search/SearchPlayableResultsScreen.vue'),
    meta: { screen: 'Search.Playables' },
  },
  {
    name: 'upload',
    path: '/upload',
    component: () => import('@/components/screens/UploadScreen.vue'),
    meta: { screen: 'Upload', guard: () => usePolicies().currentUserCan.uploadSongs() },
  },
  {
    name: 'hunt',
    path: '/discover',
    component: () => import('@/components/screens/hunting/HuntScreen.vue'),
    meta: { screen: 'Hunt' },
  },
  {
    name: 'watches',
    path: '/watches',
    component: () => import('@/components/screens/hunting/WatchesScreen.vue'),
    meta: { screen: 'Watches' },
  },
  {
    name: 'jobs',
    path: '/jobs',
    component: () => import('@/components/screens/hunting/JobsScreen.vue'),
    meta: { screen: 'Jobs' },
  },
  {
    name: 'orphans',
    path: '/orphans',
    component: () => import('@/components/screens/hunting/OrphansScreen.vue'),
    meta: { screen: 'Orphans' },
  },
  {
    name: 'settings',
    path: '/settings',
    component: () => import('@/components/screens/SettingsScreen.vue'),
    meta: { screen: 'Settings' },
    // Links from before the tab lived in the hash: `?tab=users`.
    beforeEnter: to => {
      const { tab, ...query } = to.query

      return typeof tab === 'string'
        ? { path: to.path, query, hash: `#${LEGACY_SETTINGS_TABS[tab] ?? tab}`, replace: true }
        : true
    },
  },
  {
    // Preferences live under Settings now.
    name: 'profile',
    path: '/profile',
    redirect: { path: '/settings', hash: '#preferences' },
  },
  {
    name: 'albums.show',
    path: `/albums/:id(${ALBUM_ID})`,
    component: () => import('@/components/screens/AlbumScreen.vue'),
    meta: { screen: 'Album' },
  },
  {
    // Links from before the tab lived in the hash.
    path: `/albums/:id(${ALBUM_ID})/:tab(songs|other-albums|information)`,
    redirect: to => ({ path: `/albums/${to.params.id}`, hash: `#${to.params.tab}` }),
  },
  {
    name: 'artists.show',
    path: `/artists/:id(${ARTIST_ID})`,
    component: () => import('@/components/screens/ArtistScreen.vue'),
    meta: { screen: 'Artist' },
  },
  {
    path: `/artists/:id(${ARTIST_ID})/:tab(songs|albums|information)`,
    redirect: to => ({ path: `/artists/${to.params.id}`, hash: `#${to.params.tab}` }),
  },
  {
    name: 'playlists.show',
    path: `/playlists/:id(${PLAYLIST_ID})`,
    component: () => import('@/components/screens/PlaylistScreen.vue'),
    meta: { screen: 'Playlist' },
  },
  {
    name: 'genres.index',
    path: '/genres',
    component: () => import('@/components/screens/GenreListScreen.vue'),
    meta: { screen: 'Genres' },
  },
  {
    name: 'genres.show',
    path: '/genres/:id',
    component: () => import('@/components/screens/GenreScreen.vue'),
    meta: { screen: 'Genre' },
  },
  {
    // A shared song: the queue opens with it.
    name: 'songs.queue',
    path: `/songs/:id(${SONG_ID})`,
    redirect: to => ({ path: '/queue', query: { song: to.params.id } }),
  },
  {
    name: 'verify-email',
    path: `/verify-email/:token(${EMAIL_TOKEN})`,
    component: OutsideTheShell,
    meta: { screen: 'VerifyEmail', public: true, layout: 'email-link' },
  },
  {
    name: 'reset-password',
    path: `/reset-password/:token(${EMAIL_TOKEN})`,
    component: OutsideTheShell,
    meta: { screen: 'ResetPassword', public: true, layout: 'email-link' },
  },
  {
    name: 'sso',
    path: `/sso/:code(${EMAIL_TOKEN})`,
    component: OutsideTheShell,
    meta: { screen: 'SsoComplete', public: true, layout: 'sso' },
  },
  {
    // Anything else: the 404 screen, at the address asked for.
    name: 'not-found',
    path: '/:path(.*)*',
    component: () => import('@/components/screens/NotFoundScreen.vue'),
    meta: { screen: '404' },
  },
] as const satisfies readonly RouteRecordRaw[]

export type RouteName = Extract<(typeof routes)[number], { name: string }>['name']
