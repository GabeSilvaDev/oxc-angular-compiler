// Runs the spec through Vite's client transforms, as jsdom and happy-dom do.
export default {
  name: 'client',
  viteEnvironment: 'client',
  setup() {
    return { teardown() {} }
  },
}
