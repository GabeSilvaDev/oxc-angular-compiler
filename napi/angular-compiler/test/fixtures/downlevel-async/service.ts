import { load } from './dep'

export async function run(): Promise<string> {
  return await load()
}
