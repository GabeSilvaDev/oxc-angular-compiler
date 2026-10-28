import { expect, it, vi } from 'vitest'

import { run } from './service'

// Hoisted above the imports by Vitest; the downleveled factory must still find its helper.
vi.mock('./dep', async () => {
  const value = await Promise.resolve('mocked')
  return { load: async () => value }
})

it('downlevels async and keeps an async vi.mock factory working', async () => {
  expect(await run()).toBe('mocked')
  expect(run.toString()).not.toContain('async')
})
