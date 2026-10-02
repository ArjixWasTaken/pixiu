declare module '*.vue'
declare module '*.jpg'
declare module '*.png'
declare module '*.svg'

declare type Closure<T = unknown | any> = (...args: Array<unknown | any>) => T

interface Constructable<T> {
  new (...args: any): T
}

type MaybeArray<T> = T | T[]

interface CompositeToken {
  'audio-token': string
  token: string
}

interface TwoFactorChallengeRequired {
  two_factor: true
  login_token: string
}

type LoginResponse = CompositeToken | TwoFactorChallengeRequired

type SSOProvider = 'Google' | 'OpenID Connect' | 'Reverse Proxy'

interface KoelGlobals {
  base_url: string
  build: string | null
  is_demo: boolean
  pusher: {
    readonly app_key: string
    readonly app_cluster: string
  }
  branding: Branding
  gravatar: {
    readonly url: string
    readonly default: string
  }
  mailer_configured: boolean
  accepted_audio_extensions: string[]
  demo_account?: {
    email: string
    password: string
  }
  auth_token?: CompositeToken | null
}

interface Window {
  KOEL: KoelGlobals

  RUNNING_UNIT_TESTS?: boolean

  readonly MediaMetadata: Constructable<Record<string, any>>
  createLemonSqueezy?: () => Closure

  LemonSqueezy: {
    Url: {
      Open: (url: string) => void
    }
  }
}

interface FileSystemEntry {
  createReader: () => FileSystemDirectoryReader
}

interface Branding {
  name: string
  logo: string
  cover: string
}

type EncyclopediaDisplayMode = 'aside' | 'full'
type ScreenHeaderLayout = 'expanded' | 'collapsed'

interface ContextMenuAction {
  id: string
  label: () => string
  action: () => void
}

interface AlbumTrack {
  readonly title: string
  readonly length: number
}

interface AlbumInfo {
  cover: string | null
  readonly tracks: AlbumTrack[]
  wiki?: {
    summary: string
    full: string
  }
  url?: string
}

interface ArtistInfo {
  image: string | null
  bio?: {
    summary: string
    full: string
  }
  url?: string
}

interface Artist {
  type: 'artists'
  readonly id: string
  name: string
  image: string // empty string = no image
  created_at: string
  mbid?: string | null
  is_external: boolean
  favorite: boolean
  rating: number
  /** How many albums of theirs the library has, when the list says. */
  album_count?: number
  permissions: {
    edit: boolean
  }
}

interface Album {
  type: 'albums'
  readonly id: string
  artist_id: Artist['id']
  artist_name: Artist['name']
  name: string
  cover: string // empty string = no cover
  thumbnail?: string | null
  created_at: string
  mbid?: string | null
  /** The platform it was downloaded from (`config/platforms.ts`); none for uploads. */
  source_platform?: string | null
  year: number | null
  length: number
  is_external: boolean
  favorite: boolean
  rating: number
  permissions: {
    edit: boolean
  }
}

interface IStreamable {
  readonly type: Song['type']
  readonly id: string
  favorite: boolean
  playback_state?: PlaybackState
  created_at: string
}

interface BasePlayable extends IStreamable {
  type: Song['type']
  title: string
  readonly length: number
  play_count_registered?: boolean
  play_count: number
  rating: number // 0-5, current user's rating; 0 = unrated
  play_start_time?: number
  preloaded?: boolean
  playback_state?: PlaybackState
  fmt_length?: string
}

interface Song extends BasePlayable {
  type: 'songs'
  readonly owner_id: User['id']
  album_id: Album['id']
  album_name: Album['name']
  album_cover: Album['cover']
  artist_id: Artist['id']
  artist_name: Artist['name']
  album_artist_id: Artist['id']
  album_artist_name: Artist['name']
  genre: string
  track: number | null
  disc: number
  year: number | null
  lyrics: string
  is_public: boolean
  is_external: boolean
  mbid?: string | null
  /** The platform it was downloaded from (`config/platforms.ts`); none for uploads. */
  source_platform?: string | null
  file_size?: number | null
  basename?: string
  deleted?: boolean
  /** When the user last played it. */
  played_at?: string | null
}

type Playable = Song
type Streamable = Playable

interface QueueState {
  type: 'queue-states'
  songs: Playable[]
  current_song: Playable | null
  playback_position: number
}

interface SmartPlaylistRuleGroup {
  id: string
  rules: SmartPlaylistRule[]
}

interface SmartPlaylistModel {
  name:
    | 'title'
    | 'length'
    | 'created_at'
    | 'rating'
    | 'album.name'
    | 'artist.name'
    | 'interactions.play_count'
    | 'interactions.last_played_at'
    | 'genre'
    | 'year'
  type: 'text' | 'number' | 'date'
  label: string
  unit?: 'seconds' | 'days'
}

interface SmartPlaylistOperator {
  operator:
    | 'is'
    | 'isNot'
    | 'contains'
    | 'notContain'
    | 'isBetween'
    | 'isGreaterThan'
    | 'isLessThan'
    | 'beginsWith'
    | 'endsWith'
    | 'inLast'
    | 'notInLast'
  label: string
  type?: SmartPlaylistModel['type'] // to override
  unit?: SmartPlaylistModel['unit'] // to override
  inputs?: number
}

interface SmartPlaylistRule {
  id: string
  model: SmartPlaylistModel
  operator: SmartPlaylistOperator['operator']
  value: any[]
}

interface SerializedSmartPlaylistRule {
  id: string
  model: SmartPlaylistModel['name']
  operator: SmartPlaylistOperator['operator']
  value: any[]
}

type SmartPlaylistInputTypes = Record<SmartPlaylistModel['type'], SmartPlaylistOperator[]>

interface FavoriteList {
  name: 'Favorites'
  playables: Playable[]
}

interface RecentlyPlayedList {
  name: 'Recently played'
  playables: Playable[]
}

interface PlaylistFolder {
  type: 'playlist-folders'
  readonly id: string
  name: string
  parent_id: PlaylistFolder['id'] | null
  // we don't need to keep track of the playlists here, as they can be computed using their folder_id value
}

interface Playlist {
  type: 'playlists'
  readonly id: string
  readonly owner_id: User['id']
  name: string
  description: string
  folder_id: PlaylistFolder['id'] | null
  is_smart: boolean
  rules: SmartPlaylistRuleGroup[]
  cover: string | null
  playables?: Playable[]
  permissions: {
    edit: boolean
    delete: boolean
  }
}

type PlaylistLike = Playlist | FavoriteList | RecentlyPlayedList

interface UserPreferences extends Record<string, any> {
  volume: number
  show_now_playing_notification: boolean
  repeat_mode: RepeatMode
  confirm_before_closing: boolean
  continuous_playback: boolean
  current_equalizer_preset: EqualizerPreset
  equalizer_presets: EqualizerPreset[]
  albums_view_mode: ViewMode
  artists_view_mode: ViewMode
  albums_sort_field: AlbumListSortField
  artists_sort_field: ArtistListSortField
  genres_sort_field: GenreListSortField
  albums_sort_order: SortOrder
  artists_sort_order: SortOrder
  genres_sort_order: SortOrder
  albums_favorites_only: boolean
  artists_favorites_only: boolean
  transcode_on_mobile: boolean
  transcode_quality: number
  lyrics_zoom_level: number | null
  theme?: Theme['id'] | null
  /** `null`: follow the system's light or dark mode. */
  dark_mode?: boolean | null
  active_extra_panel_tab: SideSheetTab | null
  detect_duplicate_uploads: boolean
  crossfade_duration: number
  home_blocks_order: string[]
  /** Home blocks switched off. */
  home_blocks_hidden: string[]
  /** Off: audio plays past the equalizer, its settings kept for later. */
  equalizer_enabled: boolean
}

type Ability = 'manage settings' | 'manage users' | 'manage songs'
type Role = ('admin' | 'manager' | 'user' | 'guest') & string

interface User {
  type: 'users'
  id: string
  name: string
  email: string
  is_prospect: boolean
  password?: string
  avatar: string
  role: Role
  sso_provider: SSOProvider | null
  sso_id: string | null
  preferences?: UserPreferences
  /**
   * Capabilities this user has been granted (via their role) — i.e. "what *I*
   * have the ability to do globally". Things like "manage settings" or
   * "manage songs". Only populated for the current user (the one making the
   * request); undefined for other users in a list.
   */
  abilities?: Ability[]
  /**
   * The user's personal Subsonic API key. Only populated for the current user
   * (their own /me response); never leaked through user listings.
   */
  subsonic_api_key?: string
  two_factor?: boolean
  /**
   * What the *current user* (the one making the request) is permitted to do
   * *to this user* — the result of running UserPolicy from their perspective.
   * Distinct from `abilities` above, which is the user's own globally-granted
   * capabilities. Always populated, regardless of who is being looked at.
   */
  permissions: {
    edit: boolean
    delete: boolean
  }
}

type CurrentUser = User & {
  preferences: UserPreferences
  abilities: Ability[]
  subsonic_api_key: string
  two_factor: boolean
  /** Signed in with a temporary password: must pick their own first. */
  password_change_required?: boolean
  email_verified?: boolean
  /** What they sign in with; `name` is what píxiū calls them. */
  username?: string
}

interface Interaction {
  type: 'interactions'
  readonly id: number
  readonly song_id: Playable['id']
  play_count: number
}

interface Favorite {
  readonly type: 'favorites'
  readonly favoriteable_id: string
  readonly favoriteable_type: 'playable' | 'album' | 'artist'
  readonly user_id: User['id']
  readonly created_at: string
}

interface OverlayState {
  dismissible: boolean
  type: 'loading' | 'success' | 'info' | 'warning' | 'error'
  message: string
}

interface PlayableRow {
  playable: Playable
  selected: boolean
}

interface EqualizerPreset {
  /** Present when this is a user-saved custom preset; absent on built-ins and on the modified-but-unsaved state. */
  id?: string
  name: string | null
  preamp: number
  gains: number[]
}

declare type PlaybackState = 'Stopped' | 'Playing' | 'Paused'
/** Keyed by screen name; add a screen by merging a key into this interface from another declaration file. */
interface ScreenNames {
  '404': true
  Album: true
  Albums: true
  Artist: true
  Artists: true
  Default: true
  Favorites: true
  Genre: true
  Genres: true
  Home: true
  Hunt: true
  Jobs: true
  Orphans: true
  Watches: true
  'Invitation.Accept': true
  OfflineSongs: true
  'Password.Reset': true
  Playlist: true
  Queue: true
  RecentlyPlayed: true
  ResetPassword: true
  'Search.Excerpt': true
  'Search.Playables': true
  Settings: true
  Songs: true
  SsoComplete: true
  Upload: true
  Users: true
  VerifyEmail: true
}

declare type ScreenName = keyof ScreenNames

declare type CardLayout = 'full' | 'compact'

interface AddToMenuConfig {
  queue: boolean
  favorites: boolean
}

interface PlayableListControlsConfig {
  addTo: AddToMenuConfig
  clearQueue: boolean
  refresh: boolean
}

interface Theme {
  id: string
  name: string
  thumbnail_color: string
}

type ViewMode = 'grid' | 'list' | 'table'

type RepeatMode = 'NO_REPEAT' | 'REPEAT_ALL' | 'REPEAT_ONE'

interface PlayableListConfig {
  filterable: boolean
  sortable: boolean
  reorderable: boolean
  hasCustomOrderSort: boolean
  hasHeader: boolean
}

interface PlayableListContext {
  entity?: Playlist | Album | Artist | Genre
  type?: Extract<
    ScreenName,
    | 'Home'
    | 'Songs'
    | 'Album'
    | 'Artist'
    | 'Playlist'
    | 'Favorites'
    | 'RecentlyPlayed'
    | 'Queue'
    | 'Genre'
    | 'Search.Playables'
    | 'OfflineSongs'
  >
}

type PlayableListSortField =
  | keyof Pick<
      Song,
      | 'track'
      | 'disc'
      | 'title'
      | 'album_name'
      | 'length'
      | 'artist_name'
      | 'genre'
      | 'year'
      | 'created_at'
      | 'rating'
      | 'favorite'
    >
  | 'position'

type AlbumListSortField = keyof Pick<
  Album,
  'name' | 'year' | 'artist_name' | 'created_at' | 'length' | 'rating' | 'favorite'
>
type ArtistListSortField = keyof Pick<Artist, 'name' | 'created_at' | 'rating' | 'favorite'>
type GenreListSortField = keyof Pick<Genre, 'name' | 'song_count'>
type SortField = AlbumListSortField | ArtistListSortField | GenreListSortField

interface BasicListSorterDropDownItem<T extends SortField> {
  label: string
  field: T
}

type SortOrder = 'asc' | 'desc'
type Placement = 'before' | 'after'

interface PaginateParams<S extends string = string> {
  sort: MaybeArray<S>
  order: SortOrder
  page: number
}

interface CursorPaginateParams<S extends string = string> {
  sort: MaybeArray<S>
  order: SortOrder
  cursor: string | null
}

type MethodOf<T> = { [K in keyof T]: T[K] extends Closure ? K : never }[keyof T]

interface PaginatorResource<T> {
  data: T[]
  links: {
    next: string | null
  }
  meta: {
    current_page: number
  }
}

interface CursorPaginatorResource<T> {
  data: T[]
  meta: {
    path: string
    per_page: number
    next_cursor: string | null
    prev_cursor: string | null
  }
}

type EditSongFormTabName = 'details' | 'lyrics' | 'visibility'

interface ToastMessage {
  id: string
  type: 'info' | 'success' | 'warning' | 'danger'
  content: string
  timeout: number // seconds
}

interface Genre {
  type: 'genres'
  id: string
  name: string
  song_count: number
  length: number
}

type SideSheetTab = 'Lyrics' | 'Artist' | 'Album'

type PlayableListColumnName =
  | 'title'
  | 'album'
  | 'artist'
  | 'track'
  | 'duration'
  | 'created_at'
  | 'play_count'
  | 'rating'
  | 'favorite'
  | 'year'
  | 'genre'

type AlbumTableColumnName = 'name' | 'artist' | 'time' | 'year' | 'rating' | 'favorite'

type ArtistTableColumnName = 'name' | 'rating' | 'favorite'

interface LrcLine {
  time: number
  text: string
}
