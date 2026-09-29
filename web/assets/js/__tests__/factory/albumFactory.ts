import { faker } from '@faker-js/faker'

export default (): Album => {
  return {
    type: 'albums',
    artist_id: `ar-${faker.number.int({ min: 1, max: 1_000_000 })}`,
    artist_name: faker.person.fullName(),
    id: `al-${faker.number.int({ min: 1, max: 1_000_000 })}`,
    name: faker.lorem.sentence(),
    cover: faker.image.url(),
    created_at: faker.date.past().toISOString(),
    mbid: faker.string.uuid(),
    year: faker.date.past().getFullYear(),
    is_external: false,
    favorite: faker.datatype.boolean(),
    rating: faker.number.int({ min: 0, max: 5 }),
    length: faker.number.int({ min: 60, max: 7200 }),
    permissions: {
      edit: faker.datatype.boolean(),
    },
  }
}

export const states: Record<string, Omit<Partial<Album>, 'type'>> = {
  unknown: {
    name: 'Unknown Album',
    artist_name: 'Unknown Artist',
  },
}
