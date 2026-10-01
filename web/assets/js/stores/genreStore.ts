import { defineStore } from 'pinia'
import { http } from '@/services/http'

export const useGenreStore = defineStore('genre', () => {
  const fetchAll = async () => await http.get<Genre[]>('genres')

  const fetchOne = async (id: Genre['id']) => {
    const genre = (await fetchAll()).find(genre => genre.id === id)

    if (!genre) {
      throw new Error(`No genre ${id}`)
    }

    return genre
  }

  return { fetchAll, fetchOne }
})
