import { faker } from '@faker-js/faker'
import { equalizerPresets } from '@/config/audio'

const preferences: UserPreferences = {
  volume: 0.7,
  show_now_playing_notification: false,
  repeat_mode: 'NO_REPEAT',
  confirm_before_closing: false,
  continuous_playback: false,
  equalizer: faker.helpers.arrayElement(equalizerPresets),
  current_equalizer_preset: faker.helpers.arrayElement(equalizerPresets),
  equalizer_presets: [],
  crossfade_duration: 0,
  artists_view_mode: 'grid',
  albums_view_mode: 'grid',
  albums_sort_field: 'name',
  albums_sort_order: 'asc',
  albums_favorites_only: false,
  artists_sort_field: 'name',
  artists_sort_order: 'asc',
  artists_favorites_only: false,
  genres_sort_field: 'name',
  genres_sort_order: 'asc',
  transcode_on_mobile: false,
  transcode_quality: 128,
  support_bar_no_bugging: true,
  show_album_art_overlay: true,
  lyrics_zoom_level: 1,
  theme: null,
  active_extra_panel_tab: null,
  make_uploads_public: false,
  detect_duplicate_uploads: true,
  include_public_media: true,
  home_blocks_order: [],
  home_blocks_hidden: [],
  equalizer_enabled: true,
}

export default (): User => ({
  type: 'users',
  id: faker.string.uuid(),
  name: faker.person.fullName(),
  email: faker.internet.email(),
  password: faker.internet.password(),
  is_prospect: false,
  role: 'user',
  avatar: 'https://gravatar.com/foo',
  sso_provider: null,
  sso_id: null,
  permissions: {
    edit: faker.datatype.boolean(),
    delete: faker.datatype.boolean(),
  },
})

export const states: Record<string, Omit<Partial<User>, 'type'>> = {
  admin: {
    role: 'admin',
    preferences,
    abilities: ['manage settings', 'manage users', 'manage songs'],
  },
  manager: {
    role: 'manager',
  },
  prospect: {
    is_prospect: true,
  },
  current: {
    preferences,
    abilities: [],
  },
}
