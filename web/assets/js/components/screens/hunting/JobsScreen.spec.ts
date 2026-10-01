import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { huntingService } from '@/services/huntingService'
import type { HuntJob } from '@/services/huntingService'
import Component from './JobsScreen.vue'

const job = (changes: Partial<HuntJob>): HuntJob => ({
  id: 1,
  kind: 'download',
  state: 'done',
  title: 'Somebody — Song',
  error: null,
  progress: null,
  family: null,
  created_at: '2026-09-30T00:00:00Z',
  finished_at: null,
  ...changes,
})

describe('jobsScreen.vue', () => {
  const h = createHarness()

  it('puts the failed jobs first, and counts them', async () => {
    h.mock(huntingService, 'jobs').mockResolvedValue([
      job({ id: 3, state: 'running', title: 'Somebody — Running' }),
      job({ id: 2, state: 'done', title: 'Somebody — Done' }),
      job({ id: 1, state: 'failed', title: 'Somebody — Unavailable', error: 'This video is unavailable' }),
    ])
    h.render(Component)

    await screen.findByText('Somebody — Unavailable')
    screen.getByText(/1 running · 0 waiting\s+· 1 failed/)
    const headings = screen.getAllByRole('heading').map(heading => heading.textContent?.trim())
    expect(headings.indexOf('Needs you')).toBeLessThan(headings.indexOf('Running'))
  })
})
