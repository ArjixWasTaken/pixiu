import { defineStore } from 'pinia'
import { computed, reactive } from 'vue'
import { differenceBy } from 'lodash-es'
import { http } from '@/services/http'
import { useVault } from '@/composables/useVault'

type UserFormData = Pick<User, 'name' | 'email' | 'role'>

export interface CreateUserData extends UserFormData {
  password: string
}

export interface UpdateUserData extends UserFormData {
  password?: string
}

export const useUserStore = defineStore('user', () => {
  const { vault, byId, syncWithVault } = useVault<User>()

  const state = reactive({
    users: [] as User[],
    current: null! as CurrentUser,
  })

  const current = computed(() => state.current as CurrentUser)

  const init = (currentUser: CurrentUser) => {
    state.users = syncWithVault(currentUser)
    state.current = state.users[0] as CurrentUser
  }

  const fetch = async () => {
    state.users = syncWithVault(await http.get<User[]>('users'))
  }

  const add = (user: MaybeArray<User>) => {
    state.users.push(...syncWithVault(user))
  }

  const store = async (data: CreateUserData) => {
    const user = await http.post<User>('users', data)
    add(user)
    return byId(user.id)
  }

  const update = async (user: User, data: UpdateUserData) => {
    syncWithVault(await http.put<User>(`users/${user.id}`, data))
  }

  const remove = (user: User) => {
    state.users = differenceBy(state.users, [user], 'id')
    vault.delete(user.id)
  }

  const destroy = async (user: User) => {
    await http.delete(`users/${user.id}`)
    remove(user)

    // Mama, just killed a man
    // Put a gun against his head
    // Pulled my trigger, now he's dead
    // Mama, life had just begun
    // But now I've gone and thrown it all away
    // Mama, oooh
    // Didn't mean to make you cry
    // If I'm not back again this time tomorrow
    // Carry on, carry on, as if nothing really matters
    //
    // Too late, my time has come
    // Sends shivers down my spine
    // Body's aching all the time
    // Goodbye everybody - I've got to go
    // Gotta leave you all behind and face the truth
    // Mama, oooh
    // I don't want to die
    // I sometimes wish I'd never been born at all
  }

  const regenerateSubsonicApiKey = async () => {
    const updated = await http.post<CurrentUser>('me/subsonic-api-key/regenerate')
    state.current.subsonic_api_key = updated.subsonic_api_key
    return updated.subsonic_api_key
  }

  return {
    state,
    vault,
    current,
    byId,
    syncWithVault,
    init,
    fetch,
    add,
    store,
    update,
    remove,
    destroy,
    regenerateSubsonicApiKey,
  }
})
