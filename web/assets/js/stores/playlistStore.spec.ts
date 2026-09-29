import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import factory from '@/__tests__/factory'
import { cache } from '@/services/cache'
import { http } from '@/services/http'
import { playlistStore } from '@/stores/playlistStore'

const ruleGroups: SmartPlaylistRuleGroup[] = [
  {
    id: 'c328a77e-3edf-46ed-8c8b-398ec443e6ad',
    rules: [
      {
        id: '72f58da9-7350-488a-a1ee-c7c0edbfcc99',
        model: {
          name: 'artist.name',
          type: 'text',
          label: 'Artist',
        },
        operator: 'is',
        value: ['Elvis Presley'],
      },
    ],
  },
  {
    id: '72f58da9-7350-488a-a1ee-c7c0edbfcc99',
    rules: [
      {
        id: '5d0e38c9-1eb3-40a1-b98e-23492ed01956',
        model: {
          name: 'artist.name',
          type: 'text',
          label: 'Artist',
        },
        operator: 'is',
        value: ['Queen'],
      },
    ],
  },
]

const serializedRuleGroups = [
  {
    id: 'c328a77e-3edf-46ed-8c8b-398ec443e6ad',
    rules: [
      {
        id: '72f58da9-7350-488a-a1ee-c7c0edbfcc99',
        model: 'artist.name',
        operator: 'is',
        value: ['Elvis Presley'],
      },
    ],
  },
  {
    id: '72f58da9-7350-488a-a1ee-c7c0edbfcc99',
    rules: [
      {
        id: '5d0e38c9-1eb3-40a1-b98e-23492ed01956',
        model: 'artist.name',
        operator: 'is',
        value: ['Queen'],
      },
    ],
  },
]

describe('playlistStore', () => {
  const h = createHarness()

  it('serializes playlist for storage', () => {
    expect(playlistStore.serializeSmartPlaylistRulesForStorage(ruleGroups)).toEqual(serializedRuleGroups)
  })

  it('sets up a smart playlist with properly unserialized rules', () => {
    const playlist = h.factory('playlist').make({
      is_smart: true,
      rules: serializedRuleGroups as unknown as SmartPlaylistRuleGroup[],
    })

    playlistStore.setupSmartPlaylist(playlist)

    expect(playlist.rules).toEqual(ruleGroups)
  })

  it('does not modify a smart playlist content', async () => {
    const playlist = factory('playlist').state('smart').make()
    const postMock = h.mock(http, 'post')

    await playlistStore.addContent(playlist, h.factory('song').make(3))
    expect(postMock).not.toHaveBeenCalled()

    await playlistStore.removeContent(playlist, h.factory('song').make(3))
    expect(postMock).not.toHaveBeenCalled()
  })

  it('updates a smart playlist', async () => {
    const playlist = factory('playlist').state('smart').make()
    playlistStore.state.playlists = [playlist]
    const rules = h.factory('smart-playlist-rule-group').make(2)
    const serializeMock = h.mock(playlistStore, 'serializeSmartPlaylistRulesForStorage', ['Whatever'])
    const putMock = h.mock(http, 'put').mockResolvedValue(playlist)
    const removeMock = h.mock(cache, 'remove')

    await playlistStore.update(playlist, {
      rules,
      name: 'Foo',
      description: 'Bar',
    })

    expect(serializeMock).toHaveBeenCalledWith(rules)

    expect(putMock).toHaveBeenCalledWith(`playlists/${playlist.id}`, {
      name: 'Foo',
      description: 'Bar',
      rules: ['Whatever'],
      folder_id: undefined,
    })

    expect(removeMock).toHaveBeenCalledWith(['playlist.songs', playlist.id])
  })
})
