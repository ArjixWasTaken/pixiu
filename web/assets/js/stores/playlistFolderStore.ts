import { defineStore } from 'pinia'
import type { UnwrapNestedRefs } from 'vue'
import { reactive } from 'vue'
import { http } from '@/services/http'
import { differenceBy, orderBy } from 'lodash-es'
import { usePlaylistStore } from '@/stores/playlistStore'

type PlaylistFolderUpdateData = Partial<Pick<PlaylistFolder, 'name' | 'parent_id'>>

const sort = (folders: PlaylistFolder[] | UnwrapNestedRefs<PlaylistFolder>[]) => orderBy(folders, 'name')

export const usePlaylistFolderStore = defineStore('playlistFolder', () => {
  const state = reactive<{ folders: PlaylistFolder[] }>({
    folders: [],
  })

  const init = (folders: PlaylistFolder[]) => {
    state.folders = sort(reactive(folders))
  }

  const byId = (id: PlaylistFolder['id']) => state.folders.find(folder => folder.id === id)

  const byParent = (parent: PlaylistFolder | null) => {
    const folders = parent
      ? state.folders.filter(folder => folder.parent_id === parent.id)
      : state.folders.filter(folder => folder.parent_id === null)

    return sort(folders)
  }

  const descendantsOf = (folder: PlaylistFolder) => {
    const descendants: PlaylistFolder[] = []
    const visitedFolderIds = new Set<PlaylistFolder['id']>([folder.id])
    const foldersToVisit = [...byParent(folder)].reverse()

    while (foldersToVisit.length) {
      const descendant = foldersToVisit.pop()!

      if (visitedFolderIds.has(descendant.id)) {
        continue
      }

      visitedFolderIds.add(descendant.id)
      descendants.push(descendant)
      foldersToVisit.push(...byParent(descendant).reverse())
    }

    return descendants
  }

  const pathFor = (folder: PlaylistFolder) => {
    const path: PlaylistFolder['name'][] = []
    const visitedFolderIds = new Set<PlaylistFolder['id']>()
    let currentFolder: PlaylistFolder | undefined = folder

    while (currentFolder && !visitedFolderIds.has(currentFolder.id)) {
      visitedFolderIds.add(currentFolder.id)
      path.unshift(currentFolder.name)
      currentFolder = currentFolder.parent_id ? byId(currentFolder.parent_id) : undefined
    }

    return path.join(' / ')
  }

  const playlistsInTree = (folder: PlaylistFolder) =>
    [folder, ...descendantsOf(folder)].flatMap(currentFolder => usePlaylistStore().byFolder(currentFolder))

  const store = async (name: PlaylistFolder['name'], parent?: PlaylistFolder | null) => {
    const data: { name: PlaylistFolder['name']; parent_id?: PlaylistFolder['id'] | null } = { name }

    if (parent !== undefined) {
      data.parent_id = parent ? parent.id : null
    }

    const folder = reactive(await http.post<PlaylistFolder>('playlist-folders', data))

    state.folders.push(folder)
    state.folders = orderBy(state.folders, 'name')

    return folder
  }

  const destroy = async (folder: PlaylistFolder) => {
    const childFolders = byParent(folder)
    const playlists = usePlaylistStore().byFolder(folder)

    await http.delete(`playlist-folders/${folder.id}`)

    childFolders.forEach(childFolder => {
      childFolder.parent_id = null
    })
    playlists.forEach(playlist => {
      playlist.folder_id = null
    })
    state.folders = differenceBy(state.folders, [folder], 'id')
  }

  const rename = async (folder: PlaylistFolder, name: PlaylistFolder['name']) => {
    await http.put(`playlist-folders/${folder.id}`, { name })
    byId(folder.id)!.name = name
  }

  const update = async (folder: PlaylistFolder, data: PlaylistFolderUpdateData) => {
    await http.patch(`playlist-folders/${folder.id}`, data)
    Object.assign(byId(folder.id)!, data)
  }

  const moveFolderToFolder = async (folder: PlaylistFolder, parent: PlaylistFolder | null) => {
    const parentId = parent?.id ?? null

    if (
      folder.parent_id === parentId ||
      parent?.id === folder.id ||
      (parent && descendantsOf(folder).some(descendant => descendant.id === parent.id))
    ) {
      return
    }

    await update(folder, { parent_id: parentId })
  }

  const movePlaylistToFolder = async (playlist: Playlist, folder: PlaylistFolder | null) => {
    const targetFolderId = folder?.id ?? null

    if (playlist.folder_id === targetFolderId) {
      return
    }

    const sourceFolderId = playlist.folder_id

    // Update folder_id locally so the UI reflects the move immediately.
    playlist.folder_id = targetFolderId

    try {
      if (folder) {
        await http.post(`playlist-folders/${folder.id}/playlists`, { playlists: [playlist.id] })
      } else if (sourceFolderId) {
        await http.delete(`playlist-folders/${sourceFolderId}/playlists`, { playlists: [playlist.id] })
      }
    } catch (error) {
      // Roll the optimistic mutation back so the UI doesn't diverge from the server.
      playlist.folder_id = sourceFolderId
      throw error
    }
  }

  return {
    state,
    init,
    byId,
    byParent,
    descendantsOf,
    pathFor,
    playlistsInTree,
    store,
    delete: destroy,
    rename,
    update,
    moveFolderToFolder,
    movePlaylistToFolder,
    sort,
  }
})
