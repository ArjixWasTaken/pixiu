import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { queryClient } from '@/services/queryClient'
import factory from '@/__tests__/factory'
import { huntingService } from '@/services/huntingService'
import { subsonic } from '@/services/subsonic'
import { usePlayableStore } from '@/stores/playableStore'
import { useAlbumStore } from '@/stores/albumStore'

describe('albumStore', () => {
  const h = createHarness()

  it('gets an album by ID', () => {
    const album = h.factory('album').make()
    useAlbumStore().vault.set(album.id, album)
    expect(useAlbumStore().byId(album.id)).toEqual(album)
  })

  it('removes albums by IDs', () => {
    const albums = h.factory('album').make(3)
    albums.forEach(album => useAlbumStore().vault.set(album.id, album))
    // A list of them, as a screen keeps it.
    queryClient.setQueryData(['albums', { sort: 'name' }], {
      pages: [{ items: albums, nextCursor: null }],
      pageParams: [''],
    })

    useAlbumStore().removeByIds([albums[0].id, albums[1].id])

    expect(useAlbumStore().vault.size).toBe(1)
    expect(useAlbumStore().vault.has(albums[0].id)).toBe(false)
    expect(useAlbumStore().vault.has(albums[1].id)).toBe(false)
    expect(queryClient.getQueryData<any>(['albums', { sort: 'name' }]).pages[0].items).toEqual([albums[2]])
  })

  it('identifies an unknown album', () => {
    const album = factory('album').state('unknown').make()

    expect(useAlbumStore().isUnknown(album)).toBe(true)
    expect(useAlbumStore().isUnknown(h.factory('album').make())).toBe(false)
  })

  it('syncs albums with the vault', () => {
    const album = h.factory('album').make({ name: 'IV' })

    useAlbumStore().syncWithVault(album)
    expect(useAlbumStore().vault.get(album.id)).toEqual(album)

    album.name = 'V'
    useAlbumStore().syncWithVault(album)

    expect(useAlbumStore().vault.size).toBe(1)
    expect(useAlbumStore().vault.get(album.id)?.name).toBe('V')
  })

  it('updates through píxiū, then reloads the album', async () => {
    const album = h.factory('album').make({ name: 'IV' })
    useAlbumStore().syncWithVault(album)

    const updateData = { title: 'V', artist: 'Band', year: 2010, tracks: [] }
    const editMock = h.mock(huntingService, 'editAlbum').mockResolvedValueOnce(undefined)
    h.mock(subsonic, 'album').mockResolvedValueOnce({ ...album, name: 'V', year: 2010 })
    h.mock(subsonic, 'albumSongs').mockResolvedValueOnce([])
    const syncPropsMock = h.mock(usePlayableStore(), 'syncAlbumProperties')

    await useAlbumStore().update(album, updateData)

    expect(editMock).toHaveBeenCalledWith(album, updateData)
    expect(useAlbumStore().vault.get(album.id)?.name).toBe('V')
    expect(useAlbumStore().vault.get(album.id)?.year).toBe(2010)
    expect(syncPropsMock).toHaveBeenCalled()
  })
})
