import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { playlistFolderStore } from '@/stores/playlistFolderStore'
import { playlistStore } from '@/stores/playlistStore'
import Component from './CreateSmartPlaylistForm.vue'

describe('createSmartPlaylistForm', () => {
  const h = createHarness()

  const renderComponent = (folder?: PlaylistFolder | null) => {
    playlistFolderStore.state.folders = h.factory('playlist-folder').make(2)

    return h.render(Component, {
      props: {
        folder: folder ?? null,
      },
    })
  }

  it('opens on the Details tab', () => {
    renderComponent()

    screen.getByText('New smart playlist')
    expect(screen.getByRole('tab', { name: 'Details' }).getAttribute('aria-selected')).toBe('true')
    screen.getByRole('textbox', { name: 'Name' })
    screen.getByRole('textbox', { name: 'Description' })
  })

  it('starts with one rule to fill in', async () => {
    renderComponent()

    await h.user.click(screen.getByRole('tab', { name: 'Rules' }))

    screen.getByRole('heading', { name: 'Songs that match all of these' })
    expect(screen.getAllByTestId('smart-playlist-rule')).toHaveLength(1)
  })

  it('saves the details and rules', async () => {
    const playlist = h.factory('playlist').make()
    const storeMock = h.mock(playlistStore, 'store').mockResolvedValue(playlist)
    renderComponent()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Rock Playlist')
    await h.user.click(screen.getByRole('tab', { name: 'Rules' }))
    await h.user.selectOptions(screen.getByRole('combobox', { name: 'Field' }), 'genre')
    await h.user.selectOptions(screen.getByRole('combobox', { name: 'Condition' }), 'contains')
    await h.user.type(screen.getByRole('textbox', { name: 'Value' }), 'rock')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(storeMock).toHaveBeenCalled())
    const submitted = storeMock.mock.calls[0][0]
    expect(submitted.name).toBe('Rock Playlist')
    expect(submitted.rules).toHaveLength(1)
    expect(submitted.rules[0].rules[0]).toMatchObject({ operator: 'contains', value: ['rock'] })
    expect(submitted.rules[0].rules[0].model.name).toBe('genre')
  })

  it('shows a rule left blank instead of saving', async () => {
    const storeMock = h.mock(playlistStore, 'store')
    renderComponent()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Rock Playlist')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(screen.getByRole('tab', { name: 'Rules' }).getAttribute('aria-selected')).toBe('true'))
    expect(storeMock).not.toHaveBeenCalled()
  })

  it('saves without rules once they are removed', async () => {
    const playlist = h.factory('playlist').make()
    const storeMock = h.mock(playlistStore, 'store').mockResolvedValue(playlist)
    renderComponent()

    await h.type(screen.getByRole('textbox', { name: 'Name' }), 'Empty for now')
    await h.user.click(screen.getByRole('tab', { name: 'Rules' }))
    await h.user.click(screen.getByRole('button', { name: 'Remove this rule' }))
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() => expect(storeMock).toHaveBeenCalledWith(expect.objectContaining({ rules: [] })))
  })

  it('pre-selects folder when folder prop is provided', () => {
    const folder = h.factory('playlist-folder').make()
    playlistFolderStore.state.folders = [folder]

    h.render(Component, {
      props: { folder },
    })

    // The folder select should have the folder's id selected
    const folderOption = screen.getByRole('option', { name: folder.name }) as HTMLOptionElement
    expect(folderOption.selected).toBe(true)
  })

  it('closes without confirmation when pristine', async () => {
    const { emitted } = renderComponent()

    await h.user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(emitted().close).toBeTruthy()
  })
})
