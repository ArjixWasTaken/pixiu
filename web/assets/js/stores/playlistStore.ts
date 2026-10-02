import { defineStore } from 'pinia'
import { differenceBy, orderBy } from 'lodash-es'
import { reactive } from 'vue'
import { moveItemsInList } from '@/utils/helpers'
import { logger } from '@/utils/logger'
import { uuid } from '@/utils/crypto'
import { http } from '@/services/http'
import { subsonic } from '@/services/subsonic'
import { queryClient } from '@/services/queryClient'
import { usePlaylistFolderStore } from '@/stores/playlistFolderStore'
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

/**
 * Set up a smart playlist by properly construct its structure from serialized database values.
 */
const setupSmartPlaylist = (playlist: Playlist) => {
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
}

/**
 * Serialize the rule (groups) to be storage-ready.
 */
const serializeSmartPlaylistRulesForStorage = (ruleGroups: SmartPlaylistRuleGroup[]) => {
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
}

const sort = (playlists: Playlist[]) => orderBy(playlists, ['is_smart', 'name'], ['desc', 'asc'])

const createEmptySmartPlaylistRule = (): SmartPlaylistRule => ({
  id: uuid(),
  model: models[0],
  operator: operators[0].operator,
  value: [''],
})

const createEmptySmartPlaylistRuleGroup = (): SmartPlaylistRuleGroup => ({
  id: uuid(),
  rules: [createEmptySmartPlaylistRule()],
})

export const usePlaylistStore = defineStore('playlist', () => {
  const state = reactive({
    playlists: [] as Playlist[],
  })

  const init = (playlists: Playlist[]) => {
    sort(reactive(playlists)).forEach(playlist => {
      if (!playlist.is_smart) {
        state.playlists.push(playlist)
      } else {
        try {
          setupSmartPlaylist(playlist)
          state.playlists.push(playlist)
        } catch (error: unknown) {
          logger.warn(`Failed to setup smart playlist "${playlist.name}".`, error)
        }
      }
    })
  }

  const byId = (id: Playlist['id']) => state.playlists.find(playlist => playlist.id === id)

  const byFolder = (folder: PlaylistFolder) => state.playlists.filter(({ folder_id }) => folder_id === folder.id)

  /** The folder chosen in a form: an existing one, or a new one by name. */
  const resolveFolder = async (data: { folder_id?: PlaylistFolder['id'] | null; folder_name?: string | null }) => {
    if (data.folder_name) {
      return (await usePlaylistFolderStore().store(data.folder_name)).id
    }

    return data.folder_id ?? null
  }

  const fileInFolder = async (playlist: Playlist, folderId: PlaylistFolder['id'] | null) => {
    if (playlist.folder_id === folderId) {
      return
    }

    if (folderId) {
      await http.post(`playlist-folders/${folderId}/playlists`, { playlists: [playlist.id] })
    } else if (playlist.folder_id) {
      await http.delete(`playlist-folders/${playlist.folder_id}/playlists`, { playlists: [playlist.id] })
    }

    playlist.folder_id = folderId
  }

  const store = async (data: CreatePlaylistData, songs: Playable[] = []) => {
    const folderId = await resolveFolder(data)
    let created: Playlist

    if (data.rules) {
      created = subsonic.toPlaylist(
        await http.post<Record<string, any>>('playlists', {
          name: data.name,
          description: data.description,
          folder_id: folderId,
          rules: serializeSmartPlaylistRulesForStorage(data.rules),
        }),
      )
    } else {
      created = await subsonic.createPlaylist(
        data.name,
        songs.map(song => song.id),
      )

      if (data.description) {
        await subsonic.updatePlaylist(created.id, { comment: data.description })
        created.description = data.description
      }

      await fileInFolder(created, folderId)
    }

    const playlist = reactive(created)

    if (playlist.is_smart) {
      setupSmartPlaylist(playlist)
    }

    state.playlists.push(playlist)
    state.playlists = sort(state.playlists)

    return playlist
  }

  const destroy = async (playlist: Playlist) => {
    await subsonic.deletePlaylist(playlist.id)
    state.playlists = differenceBy(state.playlists, [playlist], 'id')
  }

  const addContent = async (playlist: Playlist, playables: Playable[]) => {
    // Smart playlists pick their own songs; mirrors of watched playlists follow their platform.
    if (playlist.is_smart || !playlist.permissions.edit) {
      return playlist
    }

    await subsonic.updatePlaylist(playlist.id, { songIdToAdd: playables.map(song => song.id) })
    await queryClient.invalidateQueries({ queryKey: ['playlist', playlist.id, 'songs'] })

    return playlist
  }

  const removeContent = async (playlist: Playlist, playables: Playable[]) => {
    if (playlist.is_smart) {
      return playlist
    }

    const removed = new Set(playables.map(song => song.id))
    const current = await subsonic.playlistSongs(playlist.id)

    await subsonic.setPlaylistSongs(
      playlist.id,
      current.filter(song => !removed.has(song.id)).map(song => song.id),
    )
    await queryClient.invalidateQueries({ queryKey: ['playlist', playlist.id, 'songs'] })

    return playlist
  }

  const update = async (playlist: Playlist, data: UpdatePlaylistData) => {
    await http.put(`playlists/${playlist.id}`, {
      name: data.name,
      description: data.description,
      rules: data.rules ? serializeSmartPlaylistRulesForStorage(data.rules) : undefined,
    })

    // A form without a folder field leaves the playlist where it is.
    const folderId = data.folder_id === undefined && !data.folder_name ? playlist.folder_id : await resolveFolder(data)
    await fileInFolder(byId(playlist.id) ?? playlist, folderId)
    data = { ...data, folder_id: folderId, folder_name: undefined }

    if (playlist.is_smart) {
      await queryClient.invalidateQueries({ queryKey: ['playlist', playlist.id, 'songs'] })
    }

    Object.assign(byId(playlist.id)!, data)
  }

  const moveItemsInPlaylist = async (
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
  }

  return {
    state,
    init,
    setupSmartPlaylist,
    byId,
    byFolder,
    resolveFolder,
    fileInFolder,
    store,
    delete: destroy,
    addContent,
    removeContent,
    update,
    createEmptySmartPlaylistRule,
    createEmptySmartPlaylistRuleGroup,
    serializeSmartPlaylistRulesForStorage,
    sort,
    moveItemsInPlaylist,
  }
})
