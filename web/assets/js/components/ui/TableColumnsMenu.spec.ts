import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { albumTableColumnConfig } from '@/config/tables'
import Component from './TableColumnsMenu.vue'

describe('tableColumnsMenu.vue', () => {
  const h = createHarness()

  const columns = [
    { name: 'name', label: 'Name' },
    { name: 'artist', label: 'Artist' },
    { name: 'year', label: 'Year' },
  ]

  it('turns columns on and off, and stays open for the next', async () => {
    h.render(Component, { props: { config: albumTableColumnConfig, columns } })
    const { shouldShowColumn } = useTableColumnVisibility(albumTableColumnConfig)
    expect(shouldShowColumn('year')).toBe(true)

    await h.user.click(screen.getByRole('button', { name: 'Columns' }))
    await h.user.click(screen.getByRole('menuitemcheckbox', { name: 'Year' }))

    expect(shouldShowColumn('year')).toBe(false)
    expect(screen.getByRole('menuitemcheckbox', { name: 'Year' }).getAttribute('aria-checked')).toBe('false')
  })

  it('keeps the columns always shown', async () => {
    h.render(Component, { props: { config: albumTableColumnConfig, columns } })

    await h.user.click(screen.getByRole('button', { name: 'Columns' }))

    expect(screen.getByRole('menuitemcheckbox', { name: 'Name' }).hasAttribute('data-disabled')).toBe(true)
  })
})
