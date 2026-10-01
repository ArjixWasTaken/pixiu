import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { http } from '@/services/http'
import type { CreateUserData, UpdateUserData } from '@/stores/userStore'
import { useUserStore } from '@/stores/userStore'
describe('userStore', () => {
  let currentUser: CurrentUser

  const h = createHarness({
    beforeEach: () => {
      useUserStore().vault.clear()
      useUserStore().init(currentUser)
    },
  })

  currentUser = h.factory('user').state('current').make() as CurrentUser

  it('initializes with current user', () => {
    expect(useUserStore().current).toEqual(currentUser)
    expect(useUserStore().vault.size).toBe(1)
  })

  it('syncs with vault', () => {
    const user = h.factory('user').make()

    expect(useUserStore().syncWithVault(user)).toEqual([user])
    expect(useUserStore().vault.size).toBe(2)
    expect(useUserStore().vault.get(user.id)).toEqual(user)
  })

  it('fetches users', async () => {
    const users = h.factory('user').make(3)
    const getMock = h.mock(http, 'get').mockResolvedValue(users)

    await useUserStore().fetch()

    expect(getMock).toHaveBeenCalledWith('users')
    expect(useUserStore().vault.size).toBe(4)
  })

  it('gets user by id', () => {
    const user = h.factory('user').make()
    useUserStore().syncWithVault(user)

    expect(useUserStore().byId(user.id)).toEqual(user)
  })

  it('creates a user', async () => {
    const data: CreateUserData = {
      role: 'user',
      password: 'bratwurst',
      name: 'Jane Doe',
      email: 'jane@doe.com',
    }

    const user = h.factory('user').make(data)
    const postMock = h.mock(http, 'post').mockResolvedValue(user)

    expect(await useUserStore().store(data)).toEqual(user)
    expect(postMock).toHaveBeenCalledWith('users', data)
    expect(useUserStore().vault.size).toBe(2)
    expect(useUserStore().state.users).toHaveLength(2)
  })

  it('updates a user', async () => {
    const user = h.factory('user').make()
    useUserStore().state.users.push(...useUserStore().syncWithVault(user))

    const data: UpdateUserData = {
      role: 'admin',
      password: 'bratwurst',
      name: 'Jane Doe',
      email: 'jane@doe.com',
    }

    const updated = { ...user, ...data }
    const putMock = h.mock(http, 'put').mockResolvedValue(updated)

    await useUserStore().update(user, data)

    expect(putMock).toHaveBeenCalledWith(`users/${user.id}`, data)
    expect(useUserStore().vault.get(user.id)).toEqual(updated)
  })

  it('deletes a user', async () => {
    const deleteMock = h.mock(http, 'delete')

    const user = h.factory('user').make()
    useUserStore().state.users.push(...useUserStore().syncWithVault(user))
    expect(useUserStore().vault.has(user.id)).toBe(true)

    expect(await useUserStore().destroy(user))

    expect(deleteMock).toHaveBeenCalledWith(`users/${user.id}`)
    expect(useUserStore().vault.size).toBe(1)
    expect(useUserStore().state.users).toHaveLength(1)
    expect(useUserStore().vault.has(user.id)).toBe(false)
  })
})
