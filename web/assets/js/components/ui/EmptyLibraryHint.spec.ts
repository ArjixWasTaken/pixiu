import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { commonStore } from '@/stores/commonStore'
import Component from './EmptyLibraryHint.vue'

describe('emptyLibraryHint.vue', () => {
  const h = createHarness()

  it('asks an admin to set up the library when no media path is set', () => {
    commonStore.state.storage_driver = 'local'
    commonStore.state.media_path_set = false

    h.actingAsAdmin().render(Component)

    screen.getByText('Have you set up your library yet?')
  })

  it('invites an upload on cloud storage, where there is no media path to set', () => {
    commonStore.state.storage_driver = 's3'
    commonStore.state.media_path_set = false

    h.actingAsAdmin().render(Component)

    screen.getByText('Upload some music')
    expect(screen.queryByText('Have you set up your library yet?')).toBeNull()
  })

  it('says nothing to an uploader who cannot fix the missing media path', async () => {
    commonStore.state.storage_driver = 'local'
    commonStore.state.media_path_set = false

    await h.withPlusEdition(async () => {
      h.actingAsUser(h.factory('user').make({ abilities: [] }) as CurrentUser).render(Component)

      expect(screen.queryByText('Upload some music')).toBeNull()
      expect(screen.queryByText('Have you set up your library yet?')).toBeNull()
    })
  })

  it('says nothing to a user who can neither configure nor upload', () => {
    commonStore.state.storage_driver = 'local'
    commonStore.state.media_path_set = false

    h.actingAsUser().render(Component)

    expect(screen.queryByText('Have you set up your library yet?')).toBeNull()
    expect(screen.queryByText('Upload some music')).toBeNull()
  })
})
