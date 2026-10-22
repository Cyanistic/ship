// GitHub alerts (`> [!NOTE]`) become the ansi note: a box with a neutral crewmate standing on its edge.
// The sources stay plain markdown that GitHub renders as its own alert. The three docs crew take turns
// down the page, so neighbouring notes never show the same crewmate. A Sätteri mdast plugin, since
// Sätteri is Astro's Markdown processor.
const CREW = ["clipboard", "point", "reach"];
const KIND = /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\][ \t]*\n?/;

/** an element with any tag, built the way Starlight builds its asides: an mdast parent with hName.
 * Sätteri's types leave hName out of node data although its renderer honours it, hence the cast. */
const el = (tag, props, children = []) =>
  /** @type {import('mdast').Paragraph} */ (
    /** @type {unknown} */ ({ type: "paragraph", data: { hName: tag, hProperties: props }, children })
  );

export const alerts = {
  name: "ship-alerts",
  blockquote(node, ctx) {
    const [first, ...rest] = node.children;
    const text = first?.type === "paragraph" ? first.children[0] : undefined;
    const m = text?.type === "text" ? KIND.exec(text.value) : null;
    if (!m) return;
    const value = text.value.slice(m[0].length);
    const inline = [...(value ? [{ type: "text", value }] : []), ...first.children.slice(1)];
    const body = [...(inline.length ? [{ type: "paragraph", children: inline }] : []), ...rest];
    const n = (ctx.data.shipNotes = (ctx.data.shipNotes ?? 0) + 1) - 1;
    const kind = m[1].toLowerCase();
    return el("aside", { class: `note note-${kind}`, "aria-label": kind }, [
      el("span", { class: `act a-${CREW[n % CREW.length]}`, "aria-hidden": "true" }),
      el("div", { class: "note-body" }, body),
    ]);
  },
};
