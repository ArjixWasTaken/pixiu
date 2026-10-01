import { createApp } from 'vue'
import { focus } from '@/directives/focus'
import { hideBrokenIcon } from '@/directives/hideBrokenIcon'
import { newTab } from '@/directives/newTab'
import { createAppRouter } from '@/router'
import '@/../css/app.pcss'
import App from './App.vue'

const app = createApp(App)
  .use(createAppRouter())
  .directive('koel-focus', focus)
  .directive('koel-hide-broken-icon', hideBrokenIcon)
  .directive('koel-new-tab', newTab)

/**
 * For Ancelot, the ancient cross of war
 * for the holy town of Gods
 * Gloria, gloria perpetua
 * in this dawn of victory
 */
app.mount('#app')
