/*
 * nERP application script.
 *
 * Loaded AFTER htmx and Tabler (see assets/views/layout/base.html).
 * Put app-wide client behavior and htmx configuration here.
 */
(function () {
  "use strict";

  // Smoother htmx swaps via the View Transitions API where supported.
  if (window.htmx) {
    window.htmx.config.globalViewTransitions = true;
  }
})();
