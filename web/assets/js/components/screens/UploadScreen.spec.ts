import { describe, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { huntingService } from '@/services/huntingService'
import Component from './UploadScreen.vue'

describe('uploadScreen.vue', () => {
  const h = createHarness()

  it('says when there is nothing to review', async () => {
    h.mock(huntingService, 'offerings').mockResolvedValue([])
    h.render(Component)

    await screen.findByText('Nothing to review')
  })
})
