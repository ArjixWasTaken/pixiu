import { faker } from '@faker-js/faker'

export default (): Artist => {
  return {
    type: 'artists',
    id: `ar-${faker.number.int({ min: 1, max: 1_000_000 })}`,
    name: faker.person.fullName(),
    image: 'foo.jpg',
    created_at: faker.date.past().toISOString(),
    mbid: faker.string.uuid(),
    is_external: false,
    favorite: faker.datatype.boolean(),
    rating: faker.number.int({ min: 0, max: 5 }),
    permissions: {
      edit: faker.datatype.boolean(),
    },
  }
}

export const states: Record<string, Omit<Partial<Artist>, 'type'>> = {
  unknown: {
    name: 'Unknown Artist',
  },
  various: {
    name: 'Various Artists',
  },
}
