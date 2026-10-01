import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { huntingService } from '@/services/huntingService'
import Component from './HuntScreen.vue'

describe('huntScreen.vue', () => {
  const h = createHarness()

  it('searches for the words it was sent with', async () => {
    const search = h.mock(huntingService, 'search').mockResolvedValue({ tracks: [], albums: [] })
    await h.visit('/discover?q=lo%20%26%20behold')
    h.render(Component)

    await waitFor(() => expect(search).toHaveBeenCalledWith('lo & behold'))
    await screen.findByText(/Nothing found for “lo & behold”/)
    expect(screen.getByRole<HTMLInputElement>('searchbox').value).toBe('lo & behold')
  })

  it('waits for a search without them', async () => {
    const search = h.mock(huntingService, 'search')
    await h.visit('/discover')
    h.render(Component)

    screen.getByText('Find music on YouTube Music')
    expect(search).not.toHaveBeenCalled()
  })
})
