import { describe, expect, it, vi } from 'vite-plus/test'
import { markRaw, shallowRef } from 'vue'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ModalWrapper.vue'

const modalOptions = shallowRef<any>({
  component: null,
})

vi.mock('@/utils/helpers', async importOriginal => ({
  ...(await importOriginal<typeof import('@/utils/helpers')>()),
  requireInjection: () => modalOptions,
}))

describe('modalWrapper.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      modalOptions.value = { component: null }
    },
  })

  it('shows a modal component', async () => {
    h.render(Component)

    modalOptions.value = {
      component: markRaw(h.stub('test-modal')),
      props: {},
    }

    await waitFor(() => screen.getByTestId('test-modal'))
  })

  it('passes props to the modal component', async () => {
    h.render(Component)

    modalOptions.value = {
      component: markRaw(h.stub('test-modal')),
      props: { foo: 'bar' },
    }

    await waitFor(() => {
      const el = screen.getByTestId('test-modal')
      expect(el).toBeTruthy()
    })
  })

  it('closes modal on close event', async () => {
    h.render(Component)

    modalOptions.value = {
      component: markRaw(h.stub('test-modal')),
      props: {},
    }

    await waitFor(() => screen.getByTestId('test-modal'))

    // simulate close
    modalOptions.value = { component: null }

    await waitFor(() => expect(screen.queryByTestId('test-modal')).toBeNull())
  })

  it('closes what only shows something on Escape', async () => {
    h.render(Component)
    modalOptions.value = {
      component: markRaw({ template: '<section data-testid="info"><button>Close</button></section>' }),
      props: {},
    }
    await waitFor(() => screen.getByTestId('info'))

    await h.trigger(screen.getByRole('button', { name: 'Close' }), 'keyDown', { key: 'Escape' })

    expect(modalOptions.value.component).toBeNull()
  })

  it('leaves Escape to forms, which ask before losing changes', async () => {
    h.render(Component)
    const form = markRaw({ template: '<form data-testid="a-form"><input aria-label="Name" /></form>' })
    modalOptions.value = { component: form, props: {} }
    await waitFor(() => screen.getByTestId('a-form'))

    await h.trigger(screen.getByLabelText('Name'), 'keyDown', { key: 'Escape' })

    expect(modalOptions.value.component).toBe(form)
  })

  it('is a dialog named by its heading', async () => {
    h.render(Component)
    modalOptions.value = {
      component: markRaw({ template: '<section><header><h1>Song information</h1></header></section>' }),
      props: {},
    }

    await waitFor(() => screen.getByRole('dialog', { name: 'Song information' }))
  })

  it('keeps focus inside while open, and gives it back after', async () => {
    const opener = document.createElement('button')
    document.body.append(opener)
    opener.focus()

    h.render(Component)
    modalOptions.value = {
      component: markRaw({ template: '<section><button>First</button><button>Last</button></section>' }),
      props: {},
    }
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'First' })))

    await h.user.tab()
    await h.user.tab()
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'First' }))

    modalOptions.value = { component: null }
    await waitFor(() => expect(document.activeElement).toBe(opener))
    opener.remove()
  })
})
