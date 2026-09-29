import { differenceBy, orderBy } from 'lodash-es'
import { reactive } from 'vue'
import { moveItemsInList } from '@/utils/helpers'
import { logger } from '@/utils/logger'
import { uuid } from '@/utils/crypto'
import { subsonic } from '@/services/subsonic'
import { cache } from '@/services/cache'
import models from '@/config/smart-playlist/models'
import operators from '@/config/smart-playlist/operators'

export type CreatePlaylistData = Pick<Playlist, 'name' | 'description' | 'folder_id' | 'cover'> & {
  folder_name?: string | null
  songs?: Playable['id'][]
  rules?: SmartPlaylistRuleGroup[]
}

export interface UpdatePlaylistData {
  name: Playlist['name']
  description: Playlist['description']
  folder_id?: PlaylistFolder['id'] | null
  folder_name?: string | null
  cover?: string | null
  rules?: SmartPlaylistRuleGroup[]
}

export const playlistStore = {
  state: reactive({
    playlists: [] as Playlist[],
  }),

  init(playlists: Playlist[]) {
    this.sort(reactive(playlists)).forEach(playlist => {
      if (!playlist.is_smart) {
        this.state.playlists.push(playlist)
      } else {
        try {
          this.setupSmartPlaylist(playlist)
          this.state.playlists.push(playlist)
        } catch (error: unknown) {
          logger.warn(`Failed to setup smart playlist "${playlist.name}".`, error)
        }
      }
    })
  },

  /**
   * Set up a smart playlist by properly construct its structure from serialized database values.
   */
  setupSmartPlaylist: (playlist: Playlist) => {
    playlist.rules.forEach(group => {
      group.rules.forEach(rule => {
        const serializedRule = rule as unknown as SerializedSmartPlaylistRule
        const model = models.find(model => model.name === serializedRule.model)

        if (!model) {
          logger.error(`Invalid model ${rule.model} found in smart playlist ${playlist.name} (ID ${playlist.id})`)
          return
        }

        rule.model = model
      })
    })
  },

  byId(id: Playlist['id']) {
    return this.state.playlists.find(playlist => playlist.id === id)
  },

  byFolder(folder: PlaylistFolder) {
    return this.state.playlists.filter(({ folder_id }) => folder_id === folder.id)
  },

  async store(data: CreatePlaylistData, songs: Playable[] = []) {
    const created = await subsonic.createPlaylist(
      data.name,
      songs.map(song => song.id),
    )

    if (data.description) {
      await subsonic.updatePlaylist(created.id, { comment: data.description })
      created.description = data.description
    }

    const playlist = reactive(created)

    if (playlist.is_smart) {
      this.setupSmartPlaylist(playlist)
    }

    this.state.playlists.push(playlist)
    this.state.playlists = this.sort(this.state.playlists)

    return playlist
  },

  async delete(playlist: Playlist) {
    await subsonic.deletePlaylist(playlist.id)
    this.state.playlists = differenceBy(this.state.playlists, [playlist], 'id')
  },

  async addContent(playlist: Playlist, playables: Playable[]) {
    // Smart playlists pick their own songs; mirrors of watched playlists follow YouTube Music.
    if (playlist.is_smart || !playlist.permissions.edit) {
      return playlist
    }

    await subsonic.updatePlaylist(playlist.id, { songIdToAdd: playables.map(song => song.id) })
    cache.remove(['playlist.songs', playlist.id])

    return playlist
  },

  removeContent: async (playlist: Playlist, playables: Playable[]) => {
    if (playlist.is_smart) {
      return playlist
    }

    const removed = new Set(playables.map(song => song.id))
    const current = await subsonic.playlistSongs(playlist.id)

    await subsonic.setPlaylistSongs(
      playlist.id,
      current.filter(song => !removed.has(song.id)).map(song => song.id),
    )
    cache.remove(['playlist.songs', playlist.id])

    return playlist
  },

  async update(playlist: Playlist, data: UpdatePlaylistData) {
    await subsonic.updatePlaylist(playlist.id, { name: data.name, comment: data.description })

    if (playlist.is_smart) {
      cache.remove(['playlist.songs', playlist.id])
    }

    Object.assign(this.byId(playlist.id)!, data)
  },

  createEmptySmartPlaylistRule: (): SmartPlaylistRule => ({
    id: uuid(),
    model: models[0],
    operator: operators[0].operator,
    value: [''],
  }),

  createEmptySmartPlaylistRuleGroup(): SmartPlaylistRuleGroup {
    return {
      id: uuid(),
      rules: [this.createEmptySmartPlaylistRule()],
    }
  },

  /**
   * Serialize the rule (groups) to be storage-ready.
   */
  serializeSmartPlaylistRulesForStorage: (ruleGroups: SmartPlaylistRuleGroup[]) => {
    if (!ruleGroups || !ruleGroups.length) {
      return null
    }

    const serializedGroups = JSON.parse(JSON.stringify(ruleGroups))

    serializedGroups.forEach((group: any): void => {
      group.rules.forEach((rule: any) => {
        rule.model = rule.model.name
      })
    })

    return serializedGroups
  },

  sort: (playlists: Playlist[]) => {
    return orderBy(playlists, ['is_smart', 'name'], ['desc', 'asc'])
  },

  moveItemsInPlaylist: async (
    playlist: Playlist,
    playables: MaybeArray<Playable>,
    target: Playable,
    placement: Placement,
  ) => {
    const orderHash = JSON.stringify(playlist.playables?.map(({ id }) => id))
    playlist.playables?.splice(
      0,
      playlist.playables.length,
      ...moveItemsInList(playlist.playables, playables, target, placement),
    )

    if (orderHash !== JSON.stringify(playlist.playables?.map(({ id }) => id))) {
      await subsonic.setPlaylistSongs(
        playlist.id,
        playlist.playables!.map(({ id }) => id),
      )
    }
  },
}
