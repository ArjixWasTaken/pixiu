import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import factory from '@/__tests__/factory'
import { huntingService } from '@/services/huntingService'
import { subsonic } from '@/services/subsonic'
import { playableStore } from '@/stores/playableStore'
import { albumStore } from '@/stores/albumStore'

describe('albumStore', () => {
  const h = createHarness({
    beforeEach: () => {
      albumStore.vault.clear()
      albumStore.state.albums = []
    },
  })

  it('gets an album by ID', () => {
    const album = h.factory('album').make()
    albumStore.vault.set(album.id, album)
    expect(albumStore.byId(album.id)).toEqual(album)
  })

  it('removes albums by IDs', () => {
    const albums = h.factory('album').make(3)
    albums.forEach(album => albumStore.vault.set(album.id, album))
    albumStore.state.albums = albums

    albumStore.removeByIds([albums[0].id, albums[1].id])

    expect(albumStore.vault.size).toBe(1)
    expect(albumStore.vault.has(albums[0].id)).toBe(false)
    expect(albumStore.vault.has(albums[1].id)).toBe(false)
    expect(albumStore.state.albums).toEqual([albums[2]])
  })

  it('identifies an unknown album', () => {
    const album = factory('album').state('unknown').make()

    expect(albumStore.isUnknown(album)).toBe(true)
    expect(albumStore.isUnknown(h.factory('album').make())).toBe(false)
  })

  it('syncs albums with the vault', () => {
    const album = h.factory('album').make({ name: 'IV' })

    albumStore.syncWithVault(album)
    expect(albumStore.vault.get(album.id)).toEqual(album)

    album.name = 'V'
    albumStore.syncWithVault(album)

    expect(albumStore.vault.size).toBe(1)
    expect(albumStore.vault.get(album.id)?.name).toBe('V')
  })

  it('updates through píxiū, then reloads the album', async () => {
    const album = h.factory('album').make({ name: 'IV' })
    albumStore.syncWithVault(album)

    const updateData = { title: 'V', artist: 'Band', year: 2010, tracks: [] }
    const editMock = h.mock(huntingService, 'editAlbum').mockResolvedValueOnce(undefined)
    h.mock(subsonic, 'album').mockResolvedValueOnce({ ...album, name: 'V', year: 2010 })
    h.mock(subsonic, 'albumSongs').mockResolvedValueOnce([])
    const syncPropsMock = h.mock(playableStore, 'syncAlbumProperties')

    await albumStore.update(album, updateData)

    expect(editMock).toHaveBeenCalledWith(album, updateData)
    expect(albumStore.vault.get(album.id)?.name).toBe('V')
    expect(albumStore.vault.get(album.id)?.year).toBe(2010)
    expect(syncPropsMock).toHaveBeenCalled()
  })
})
