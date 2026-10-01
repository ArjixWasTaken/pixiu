import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { usePlaylistFolderStore } from '@/stores/playlistFolderStore'
import Component from './FolderSelect.vue'

describe('folderSelect', () => {
  const h = createHarness()

  const renderComponent = (folderId: PlaylistFolder['id'] | null = null) => {
    usePlaylistFolderStore().state.folders = h.factory('playlist-folder').make(3)

    return h.render(Component, {
      props: {
        folderId,
        'onUpdate:folderId': (value: any) => value,
        folderName: null,
        'onUpdate:folderName': (value: any) => value,
      },
    })
  }

  it('renders existing folders in the dropdown', () => {
    renderComponent()

    const options = screen.getAllByRole('option')
    // no folder + 3 folders + a new one
    expect(options).toHaveLength(5)
    expect(options[options.length - 1].textContent).toContain('New folder…')
  })

  it('renders existing folders with their full paths', () => {
    const root = h.factory('playlist-folder').make({ name: 'Music', parent_id: null })
    const child = h.factory('playlist-folder').make({ name: 'Live', parent_id: root.id })
    const grandchild = h.factory('playlist-folder').make({ name: '2026', parent_id: child.id })
    const earlierRoot = h.factory('playlist-folder').make({ name: 'Archive', parent_id: null })
    usePlaylistFolderStore().init([root, child, grandchild, earlierRoot])

    h.render(Component, {
      props: {
        folderId: null,
        'onUpdate:folderId': (value: any) => value,
        folderName: null,
        'onUpdate:folderName': (value: any) => value,
      },
    })

    expect(screen.getAllByRole('option').map(option => option.textContent?.trim())).toEqual([
      'None',
      'Archive',
      'Music',
      'Music / Live',
      'Music / Live / 2026',
      'New folder…',
    ])
  })

  it('switches to input mode when a new folder is chosen', async () => {
    renderComponent()

    await h.user.selectOptions(screen.getByRole('combobox'), '__new__')

    await waitFor(() => {
      screen.getByPlaceholderText('Folder name')
      screen.getByTitle('Create')
      screen.getByTitle('Cancel')
    })
  })

  it('emits folder name on confirm', async () => {
    const { emitted } = renderComponent()

    await h.user.selectOptions(screen.getByRole('combobox'), '__new__')

    await waitFor(() => screen.getByPlaceholderText('Folder name'))
    await h.user.type(screen.getByPlaceholderText('Folder name'), 'My Folder')
    await h.user.click(screen.getByTitle('Create'))

    await waitFor(() => {
      expect(emitted()['update:folderName']).toBeTruthy()
      const lastEmit = emitted()['update:folderName'].at(-1)
      expect(lastEmit).toEqual(['My Folder'])
    })
  })

  it('reverts to dropdown on cancel', async () => {
    renderComponent()

    await h.user.selectOptions(screen.getByRole('combobox'), '__new__')

    await waitFor(() => screen.getByPlaceholderText('Folder name'))
    await h.user.click(screen.getByTitle('Cancel'))

    await waitFor(() => {
      screen.getByRole('combobox')
    })
  })

  it('does not confirm when folder name is empty', async () => {
    const { emitted } = renderComponent()

    await h.user.selectOptions(screen.getByRole('combobox'), '__new__')

    await waitFor(() => screen.getByPlaceholderText('Folder name'))
    await h.user.click(screen.getByTitle('Create'))

    expect(emitted()['update:folderName']).toBeFalsy()
  })

  it('clears folder name when selecting an existing folder', async () => {
    usePlaylistFolderStore().state.folders = h.factory('playlist-folder').make(3)
    const folders = usePlaylistFolderStore().state.folders

    const { emitted } = h.render(Component, {
      props: {
        folderId: null,
        'onUpdate:folderId': (value: any) => value,
        folderName: 'Pending Folder',
        'onUpdate:folderName': (value: any) => value,
      },
    })

    await h.user.selectOptions(screen.getByRole('combobox'), folders[0].id)

    await waitFor(() => {
      const lastFolderNameEmit = emitted()['update:folderName'].at(-1)
      expect(lastFolderNameEmit).toEqual([null])
      const lastFolderIdEmit = emitted()['update:folderId'].at(-1)
      expect(lastFolderIdEmit).toEqual([folders[0].id])
    })
  })
})
