import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { Plugin } from 'vite'
import { createServer } from 'vite'
import { afterAll, describe, expect, it } from 'vitest'
import { startVitest } from 'vitest/node'

import { angular, type PluginOptions } from '../vite-plugin/index.js'

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), 'fixtures/downlevel-async')

const SOURCE = `export class A {
  b = 1;
  async f() {
    for await (const x of this.xs()) await x;
  }
}
export const g = async () => 1;
`

const roots: string[] = []
afterAll(() => roots.forEach((root) => rmSync(root, { recursive: true, force: true })))

function tempRoot(withZoneJs: boolean): string {
  // realpath.native: the dev server only serves files under the real root, and
  // Windows' temp dir can be an 8.3 short path (`RUNNER~1`).
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'oxc-angular-downlevel-async-')))
  roots.push(root)
  if (withZoneJs) {
    const zone = join(root, 'node_modules/zone.js')
    mkdirSync(zone, { recursive: true })
    writeFileSync(join(zone, 'package.json'), '{"name":"zone.js","main":"zone.js"}')
    writeFileSync(join(zone, 'zone.js'), '')
  }
  return root
}

async function transform(
  options: PluginOptions,
  { root = tempRoot(false), ssr = false, id = '/src/a.ts', code = SOURCE } = {},
): Promise<string | undefined> {
  const plugin = angular({ liveReload: false, sourceMap: false, ...options }).find(
    (candidate) => candidate.name === '@oxc-angular/vite-downlevel-async',
  ) as Plugin & { configResolved: (config: unknown) => void; transform: { handler: Function } }
  plugin.configResolved({ root })
  const result = await plugin.transform.handler.call(
    {
      error(message: string) {
        throw new Error(message)
      },
    },
    code,
    id,
    { ssr },
  )
  return result?.code
}

describe('async downleveling for zone.js', () => {
  it('downlevels async functions, async generators and for await to generators', async () => {
    const code = await transform({ zoneless: false })
    expect(code).not.toMatch(/\basync\b|for await/)
    expect(code).toContain(
      'import _asyncToGenerator from "@oxc-project/runtime/helpers/asyncToGenerator"',
    )
  })

  it('only downlevels async: the class field of the already-stripped TypeScript stays a field', async () => {
    const code = await transform({ zoneless: false })
    expect(code).toMatch(/^\s*b = 1;/m)
    expect(code).not.toContain('this.b')
  })

  it('imports the helpers as a module in a file without import or export', async () => {
    const code = await transform({ zoneless: false }, { code: 'async function f() {}\nf();\n' })
    expect(code).toContain('import _asyncToGenerator from')
    expect(code).not.toContain('require(')
  })

  it('reports a downleveling error by its message', async () => {
    await expect(
      transform({ zoneless: false }, { id: '/src/a.js', code: 'async function f( {' }),
    ).rejects.toThrow('Expected `}` but found `EOF`')
  })

  it('downlevels before import analysis, so the dev server resolves the helper import', async () => {
    const root = tempRoot(false)
    mkdirSync(join(root, 'src'))
    writeFileSync(
      join(root, 'src/main.ts'),
      'export async function f(): Promise<number> {\n  return 1\n}\n',
    )
    const server = await createServer({
      root,
      configFile: false,
      logLevel: 'silent',
      server: { middlewareMode: true, ws: false },
      plugins: [angular({ zoneless: false, liveReload: false })],
    })
    try {
      const result = await server.transformRequest('/src/main.ts')
      expect(result?.code).toContain('_asyncToGenerator')
      expect(result?.code).not.toContain('from "@oxc-project/runtime')
    } finally {
      await server.close()
    }
  })

  it('keeps async native when zoneless', async () => {
    expect(await transform({ zoneless: true })).toBeUndefined()
  })

  it('keeps async native in server (SSR) transforms', async () => {
    expect(await transform({ zoneless: false }, { ssr: true })).toBeUndefined()
  })

  it('defaults to downleveling when zone.js resolves from the root, and to native otherwise', async () => {
    expect(await transform({}, { root: tempRoot(true) })).toContain('_asyncToGenerator')
    expect(await transform({}, { root: tempRoot(false) })).toBeUndefined()
  })

  it('runs after Vitest hoists vi.mock, so an async mock factory still finds its helper', async () => {
    const vitest = await startVitest(
      'test',
      [],
      {
        root: fixtureRoot,
        config: false,
        include: ['*.check.ts'],
        environment: './client-environment.ts',
        watch: false,
        reporters: [],
      },
      { plugins: [angular({ zoneless: false, liveReload: false })] },
    )
    const files = vitest.state.getFiles()
    await vitest.close()
    const errors = files.flatMap((file) => [
      ...(file.result?.errors ?? []),
      ...file.tasks.flatMap((task) => task.result?.errors ?? []),
    ])
    expect(errors.map((error) => error.message)).toEqual([])
    expect(files.map((file) => file.result?.state)).toEqual(['pass'])
  }, 60_000)
})
