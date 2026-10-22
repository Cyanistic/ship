// Day or night. Inlined into the <head> of every page (the landing, the 404, and the docs through
// ThemeProvider) so html[data-mode] is set before the first paint. It follows the OS until a .mode toggle
// stores a choice in localStorage, which then holds on every page and in every open tab.
// public/css/mode.css draws both modes and the toggle.
export const modeScript = `(() => {
  const root = document.documentElement, key = 'ship-mode';
  const os = matchMedia('(prefers-color-scheme: dark)'), still = matchMedia('(prefers-reduced-motion: reduce)');
  let chosen = null;
  try { chosen = localStorage.getItem(key); } catch {}
  const pick = () => (chosen === 'dark' || chosen === 'light' ? chosen : os.matches ? 'dark' : 'light');
  const mark = () => document.querySelectorAll('.mode').forEach((b) => b.setAttribute('aria-pressed', String(root.dataset.mode === 'dark')));
  const show = (mode) => {
    if (mode === root.dataset.mode) return;
    const swap = () => { root.dataset.mode = mode; mark(); };
    if (document.startViewTransition && !still.matches) document.startViewTransition(swap); else swap();
  };
  root.dataset.mode = pick();
  os.addEventListener('change', () => show(pick()));
  addEventListener('storage', (e) => { if (e.key === key) { chosen = e.newValue; show(pick()); } });
  document.addEventListener('DOMContentLoaded', () => {
    mark();
    document.querySelectorAll('.mode').forEach((b) => b.addEventListener('click', () => {
      chosen = root.dataset.mode === 'dark' ? 'light' : 'dark';
      try { localStorage.setItem(key, chosen); } catch {}
      show(chosen);
    }));
  });
})();`;

/** The toggle, for raw HTML (the landing body). The Astro pages write the same markup. */
export const modeButton =
  '<button class="mode" type="button" aria-pressed="false" title="dark mode"><span class="sr-only">dark mode</span></button>';
