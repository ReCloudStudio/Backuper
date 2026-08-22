export default defineNuxtConfig({
  modules: ['@nuxt/ui'],
  devtools: { enabled: true },
  nitro: {
    routeRules: {
      '/api/**': { proxy: 'http://127.0.0.1:8080/api/**' }
    }
  }
})
