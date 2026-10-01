import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { playlistFolderStore } from '@/stores/playlistFolderStore'
import { playlistStore } from '@/stores/playlistStore'
import { eventBus } from '@/utils/eventBus'
import models from '@/config/smart-playlist/models'
import Component from './EditSmartPlaylistForm.vue'

describe('editSmartPlaylistForm', () => {
  const h = createHarness()

  const titleModel = models.find(m => m.name === 'title')!

  const createValidRuleGroup = (): SmartPlaylistRuleGroup => ({
    id: crypto.randomUUID(),
    rules: [
      {
        id: crypto.randomUUID(),
        model: titleModel,
        operator: 'contains',
        value: ['rock'],
      },
    ],
  })

  const createSmartPlaylist = (overrides: Partial<Playlist> = {}): Playlist => {
    return {
      ...h.factory('playlist').make(),
      is_smart: true,
      rules: [createValidRuleGroup()],
      ...overrides,
    } as Playlist
  }

  const renderComponent = (playlist?: Playlist) => {
    playlist = playlist ?? createSmartPlaylist()
    playlistFolderStore.state.folders = h.factory('playlist-folder').make(2)
    playlistStore.state.playlists = [playlist]

    return {
      ...h.render(Component, {
        props: {
          playlist,
        },
      }),
      playlist,
    }
  }

  it('populates form fields from existing playlist', () => {
    const { playlist } = renderComponent(createSmartPlaylist({ name: 'My Smart Playlist' }))

    expect((screen.getByRole('textbox', { name: 'Name' }) as HTMLInputElement).value).toBe(playlist.name)
  })

  it('shows existing rule groups on the Rules tab', async () => {
    renderComponent()

    await h.user.click(screen.getByRole('tab', { name: 'Rules' }))

    screen.getByRole('heading', { name: 'Songs that match all of these' })
    expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Value' }).value).toBe('rock')
  })

  it('submits changes with name and rules to playlistStore.update', async () => {
    const updateMock = h.mock(playlistStore, 'update')
    const { playlist } = renderComponent()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Updated Playlist')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(updateMock).toHaveBeenCalled()
      const [targetPlaylist, submittedData] = updateMock.mock.calls[0]
      expect(targetPlaylist.id).toBe(playlist.id)
      expect(submittedData.name).toBe('Updated Playlist')
      expect(submittedData.rules).toHaveLength(1)
      expect(submittedData.rules[0].rules).toHaveLength(1)
      expect(submittedData.rules[0].rules[0].model.name).toBe('title')
      expect(submittedData.rules[0].rules[0].operator).toBe('contains')
      expect(submittedData.rules[0].rules[0].value).toEqual(['rock'])
    })
  })

  it('omits cover from payload when unchanged', async () => {
    const updateMock = h.mock(playlistStore, 'update')
    renderComponent()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Updated')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(updateMock).toHaveBeenCalled()
      const submittedData = updateMock.mock.calls[0][1]
      expect(submittedData).not.toHaveProperty('cover')
    })
  })

  it('emits PLAYLIST_UPDATED on successful update', async () => {
    h.mock(playlistStore, 'update').mockResolvedValue(undefined)
    const emitMock = h.mock(eventBus, 'emit')

    const { playlist } = renderComponent()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Updated')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(emitMock).toHaveBeenCalledWith('PLAYLIST_UPDATED', playlist)
    })
  })

  it('saves another group of rules', async () => {
    const updateMock = h.mock(playlistStore, 'update')
    renderComponent()

    await h.user.click(screen.getByRole('tab', { name: 'Rules' }))
    await h.user.click(screen.getByRole('button', { name: /Add another group/ }))
    await h.user.type(screen.getAllByRole('textbox', { name: 'Value' })[1], 'jazz')

    await h.user.click(screen.getByRole('tab', { name: 'Details' }))
    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Updated')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => {
      expect(updateMock).toHaveBeenCalled()
      const submittedData = updateMock.mock.calls[0][1]
      expect(submittedData.rules).toHaveLength(2)
    })
  })

  it('closes without confirmation when form is pristine', async () => {
    const { emitted } = renderComponent()

    await h.user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(emitted().close).toBeTruthy()
  })
})
