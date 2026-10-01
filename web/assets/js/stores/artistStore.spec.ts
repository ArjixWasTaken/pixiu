import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { queryClient } from '@/services/queryClient'
import factory from '@/__tests__/factory'
import { http } from '@/services/http'
import { usePlayableStore } from '@/stores/playableStore'
import { useArtistStore } from '@/stores/artistStore'

describe('artistStore', () => {
  const h = createHarness()

  it('gets an artist by ID', () => {
    const artist = h.factory('artist').make()
    useArtistStore().vault.set(artist.id, artist)
    expect(useArtistStore().byId(artist.id)).toEqual(artist)
  })

  it('removes artists by IDs', () => {
    const artists = h.factory('artist').make(3)
    artists.forEach(artist => useArtistStore().vault.set(artist.id, artist))
    // A list of them, as a screen keeps it.
    queryClient.setQueryData(['artists', { sort: 'name' }], {
      pages: [{ items: artists, nextCursor: null }],
      pageParams: [''],
    })

    useArtistStore().removeByIds([artists[0].id, artists[1].id])

    expect(useArtistStore().vault.size).toBe(1)
    expect(useArtistStore().vault.has(artists[0].id)).toBe(false)
    expect(useArtistStore().vault.has(artists[1].id)).toBe(false)
    expect(queryClient.getQueryData<any>(['artists', { sort: 'name' }]).pages[0].items).toEqual([artists[2]])
  })

  it('identifies an unknown artist', () => {
    const artist = factory('artist').state('unknown').make()

    expect(useArtistStore().isUnknown(artist)).toBe(true)
    expect(useArtistStore().isUnknown(artist.name)).toBe(true)
    expect(useArtistStore().isUnknown(h.factory('artist').make())).toBe(false)
  })

  it('identifies the various artist', () => {
    const artist = factory('artist').state('various').make()

    expect(useArtistStore().isVarious(artist)).toBe(true)
    expect(useArtistStore().isVarious(artist.name)).toBe(true)
    expect(useArtistStore().isVarious(h.factory('artist').make())).toBe(false)
  })

  it('identifies a standard artist', () => {
    expect(useArtistStore().isStandard(factory('artist').state('unknown').make())).toBe(false)
    expect(useArtistStore().isStandard(factory('artist').state('various').make())).toBe(false)
    expect(useArtistStore().isStandard(h.factory('artist').make())).toBe(true)
  })

  it('syncs artists with the vault', () => {
    const artist = h.factory('artist').make({ name: 'Led Zeppelin' })

    useArtistStore().syncWithVault(artist)
    expect(useArtistStore().vault.get(artist.id)).toEqual(artist)

    artist.name = 'Pink Floyd'
    useArtistStore().syncWithVault(artist)

    expect(useArtistStore().vault.size).toBe(1)
    expect(useArtistStore().vault.get(artist.id)?.name).toBe('Pink Floyd')
  })

  it('updates artist', async () => {
    const artist = h.factory('artist').make({ name: 'Led Zeppelin' })
    useArtistStore().syncWithVault(artist)

    const updatedArtist = {
      ...artist,
      name: 'Pink Floyd',
      image: 'foo',
    }

    const updateData = {
      name: 'Pink Floyd',
      image: 'foo',
    }

    const putMock = h.mock(http, 'put').mockResolvedValue(updatedArtist)
    const syncPropsMock = h.mock(usePlayableStore(), 'syncArtistProperties')

    await useArtistStore().update(artist, updateData)

    expect(putMock).toHaveBeenCalledWith(`artists/${artist.id}`, updateData)
    expect(syncPropsMock).toHaveBeenCalledWith(updatedArtist)
  })
})
