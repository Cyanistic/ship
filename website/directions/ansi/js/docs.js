/* Docs page helpers. The page reads and navigates without any of this.

   PROVISIONAL SEARCH. This only searches the text of this one page. The real docs need an
   index built when the site is published, and the static site generator is not chosen yet. */
(() => {
  const q = document.getElementById('q');
  const box = document.getElementById('results');
  const doc = document.getElementById('main');
  if (!q || !box || !doc) return;

  // every block of text on the page, tied to the nearest heading above it
  const blocks = [];
  let head = { id: 'configuration', text: 'configuration' };
  doc.querySelectorAll('h1, h2, p, li, pre').forEach(el => {
    if (el.closest('.contents')) return;
    const text = el.textContent.replace(/[ \t]+/g, ' ').replace(/\s*\n\s*/g, '\n').trim();
    if (el.matches('h1, h2')) head = { id: el.id, text: el.firstChild.textContent.trim() };
    blocks.push({ head, text, lower: text.toLowerCase() });
  });

  const status = Object.assign(document.createElement('p'), { className: 'sr-only' });
  status.setAttribute('role', 'status');
  document.body.append(status);

  const links = () => [...box.querySelectorAll('a')];
  const hide = () => { box.hidden = true; q.setAttribute('aria-expanded', 'false'); };

  function snippet(block, terms) {
    const at = Math.max(0, block.lower.indexOf(terms[0]));
    const from = Math.max(0, at - 45);
    let s = block.text.slice(from, at + terms[0].length + 90).replace(/\s*\n\s*/g, ' ');
    return (from > 0 ? '… ' : '') + s + (from + s.length < block.text.length ? ' …' : '');
  }
  function marked(s, terms) {
    const frag = document.createDocumentFragment();
    const rx = new RegExp('(' + terms.map(t => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|') + ')', 'gi');
    s.split(rx).forEach((part, i) => {
      if (i % 2) { const m = document.createElement('mark'); m.textContent = part; frag.append(m); }
      else frag.append(part);
    });
    return frag;
  }

  function render() {
    const terms = q.value.toLowerCase().split(/\s+/).filter(Boolean);
    box.replaceChildren();
    if (!terms.length) { hide(); status.textContent = ''; return; }
    const seen = new Set();
    const hits = [];
    for (const b of blocks) {
      if (!terms.every(t => b.lower.includes(t))) continue;
      const key = b.head.id + '|' + b.text.slice(0, 60);
      if (seen.has(key)) continue;
      seen.add(key);
      hits.push(b);
    }
    hits.slice(0, 8).forEach(b => {
      const a = document.createElement('a');
      a.href = '#' + b.head.id;
      const t = document.createElement('b'); t.textContent = b.head.text;
      const s = document.createElement('span'); s.append(marked(snippet(b, terms), terms));
      a.append(t, s);
      box.append(a);
    });
    if (!hits.length) {
      const p = document.createElement('p'); p.className = 'empty';
      p.textContent = 'nothing on this page matches “' + q.value.trim() + '”.';
      box.append(p);
    }
    const note = document.createElement('p'); note.className = 'scope';
    note.textContent = 'this searches the current page only, for now.';
    box.append(note);
    box.hidden = false;
    q.setAttribute('aria-expanded', 'true');
    status.textContent = hits.length + (hits.length === 1 ? ' match' : ' matches') + ' on this page';
  }

  q.setAttribute('aria-expanded', 'false');
  q.addEventListener('input', render);
  q.addEventListener('focus', () => { if (q.value.trim()) render(); });
  q.addEventListener('keydown', e => {
    if (e.key === 'ArrowDown' && !box.hidden) { e.preventDefault(); links()[0]?.focus(); }
    else if (e.key === 'Enter') { e.preventDefault(); links()[0]?.click(); }
    else if (e.key === 'Escape') { if (q.value) { q.value = ''; render(); } else q.blur(); }
  });
  box.addEventListener('keydown', e => {
    const all = links(), i = all.indexOf(document.activeElement);
    if (e.key === 'ArrowDown') { e.preventDefault(); all[Math.min(all.length - 1, i + 1)]?.focus(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); if (i <= 0) q.focus(); else all[i - 1].focus(); }
    else if (e.key === 'Escape') { q.focus(); hide(); }
  });
  box.addEventListener('click', e => { if (e.target.closest('a')) { hide(); q.blur(); } });
  document.addEventListener('click', e => { if (!box.hidden && !q.form.contains(e.target)) hide(); });
  document.addEventListener('keydown', e => {
    if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey) return;
    const t = e.target;
    if (t.closest && t.closest('input, textarea, select, [contenteditable]')) return;
    e.preventDefault(); q.focus(); q.select();
  });

  // say where you are in the on-page contents: the last heading above the upper third of the window,
  // and the last one once the page is scrolled to the bottom (it may never reach the upper third)
  const toc = new Map([...document.querySelectorAll('.toc a[href^="#"]')].map(a => [a.hash.slice(1), a]));
  const heads = [...toc.keys()].map(id => document.getElementById(id)).filter(Boolean);
  let queued = false;
  function mark() {
    queued = false;
    let cur = heads[0];
    for (const h of heads) if (h.getBoundingClientRect().top <= innerHeight * 0.3) cur = h;
    if (innerHeight + scrollY >= document.documentElement.scrollHeight - 4) cur = heads[heads.length - 1];
    toc.forEach((a, id) => id === cur.id ? a.setAttribute('aria-current', 'location') : a.removeAttribute('aria-current'));
  }
  if (heads.length) {
    addEventListener('scroll', () => { if (!queued) { queued = true; requestAnimationFrame(mark); } }, { passive: true });
    mark();
  }
})();
