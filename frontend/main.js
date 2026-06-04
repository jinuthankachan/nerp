// nERP frontend entry — bundled by Vite into assets/static/dist/app.{js,css}.
import htmx from 'htmx.org'
window.htmx = htmx

import '@tabler/core' // Tabler behaviors (dropdowns, tooltips, …)
import '@tabler/icons-webfont/dist/tabler-icons.min.css' // icon webfont (Vite bundles the fonts)
import './main.scss' // Tabler core (themed) + nERP styles

// App-wide htmx configuration.
htmx.config.globalViewTransitions = true
