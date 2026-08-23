export default defineNuxtConfig({
  ssr: false,
  modules: ['@nuxt/ui'],
  devtools: { enabled: true },
  nitro: {
    preset: 'static',
    routeRules: {
      '/api/**': { proxy: 'http://127.0.0.1:8080/api/**' }
    }
  }
})
