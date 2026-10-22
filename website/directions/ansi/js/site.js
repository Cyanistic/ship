/* Small, optional helpers. The page reads and navigates without any of this. */
(() => {
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;

  // copy buttons on command blocks: the button says so, and so does a status line for a screen reader
  const said = Object.assign(document.createElement('p'), { className: 'sr-only' });
  said.setAttribute('role', 'status');
  document.body.append(said);
  document.querySelectorAll('.copy').forEach(b => b.addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(b.dataset.copy); b.textContent = 'copied'; b.classList.add('done'); said.textContent = 'copied to the clipboard'; }
    catch { b.textContent = 'select and copy'; }
    setTimeout(() => { b.textContent = 'copy'; b.classList.remove('done'); said.textContent = ''; }, 1600);
  }));

  // the places menu: close on choosing, on Esc and on a click elsewhere
  const places = document.querySelector('.places');
  if (places) {
    places.querySelectorAll('a').forEach(a => a.addEventListener('click', () => { places.open = false; }));
    document.addEventListener('keydown', e => {
      if (e.key === 'Escape' && places.open) { places.open = false; places.querySelector('summary').focus(); }
    });
    document.addEventListener('click', e => { if (places.open && !places.contains(e.target)) places.open = false; });

    // say where you are: the section under the upper part of the window
    const here = places.querySelector('[data-here]');
    const links = [...places.querySelectorAll('ol a')];
    const byId = new Map(links.map(a => [a.hash.slice(1), a]));
    const spy = new IntersectionObserver(entries => {
      entries.forEach(en => {
        if (!en.isIntersecting) return;
        const a = byId.get(en.target.id);
        if (!a) return;
        links.forEach(l => l.removeAttribute('aria-current'));
        a.setAttribute('aria-current', 'location');
        here.textContent = a.querySelector('.nau').textContent + ' · ' + a.querySelector('.pl').textContent;
      });
    }, { rootMargin: '-35% 0px -60% 0px' });
    byId.forEach((_, id) => { const s = document.getElementById(id); if (s) spy.observe(s); });
  }

  // a neutral crewmate glances up when its room scrolls into view, then goes back to its work
  if (!reduce && 'IntersectionObserver' in window) {
    const seen = new IntersectionObserver(entries => entries.forEach(en => {
      if (!en.isIntersecting) return;
      const el = en.target;
      el.classList.add('looking');
      setTimeout(() => el.classList.remove('looking'), 2200);
      seen.unobserve(el);
    }), { threshold: 0.6 });
    document.querySelectorAll('.room, .nest').forEach(el => seen.observe(el));
  }

  // a loop that is off screen has nobody to look at it: pause it until it comes back
  if ('IntersectionObserver' in window) {
    const watch = new IntersectionObserver(entries => entries.forEach(en => en.target.classList.toggle('off', !en.isIntersecting)));
    document.querySelectorAll('.act').forEach(el => watch.observe(el));
  }
})();
