/* Small, optional helpers. The page reads and navigates without any of this. */
(() => {
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;

  // copy buttons on command blocks: the button says so, and so does a status line for a screen reader
  const said = Object.assign(document.createElement("p"), { className: "sr-only" });
  said.setAttribute("role", "status");
  document.body.append(said);
  let saidTimer;
  const say = (text, ms) => {
    clearTimeout(saidTimer);
    said.textContent = text;
    saidTimer = setTimeout(() => (said.textContent = ""), ms);
  };
  document.querySelectorAll(".copy").forEach((b) =>
    b.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(b.dataset.copy);
        b.textContent = "copied";
        b.classList.add("done");
        said.textContent = "copied to the clipboard";
      } catch {
        b.textContent = "select and copy";
      }
      setTimeout(() => {
        b.textContent = "copy";
        b.classList.remove("done");
        said.textContent = "";
      }, 1600);
    }),
  );

  // the places menu: close on choosing, on Esc and on a click elsewhere
  const places = document.querySelector(".places");
  if (places) {
    places.querySelectorAll("a").forEach((a) =>
      a.addEventListener("click", () => {
        places.open = false;
      }),
    );
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && places.open) {
        places.open = false;
        places.querySelector("summary").focus();
      }
    });
    document.addEventListener("click", (e) => {
      if (places.open && !places.contains(e.target)) places.open = false;
    });

    // say where you are: the section under the upper part of the window
    const here = places.querySelector("[data-here]");
    const links = [...places.querySelectorAll("ol a")];
    const byId = new Map(links.map((a) => [a.hash.slice(1), a]));
    const spy = new IntersectionObserver(
      (entries) => {
        entries.forEach((en) => {
          if (!en.isIntersecting) return;
          const a = byId.get(en.target.id);
          if (!a) return;
          links.forEach((l) => l.removeAttribute("aria-current"));
          a.setAttribute("aria-current", "location");
          here.textContent = a.querySelector(".nau").textContent + " · " + a.querySelector(".pl").textContent;
        });
      },
      { rootMargin: "-35% 0px -60% 0px" },
    );
    byId.forEach((_, id) => {
      const s = document.getElementById(id);
      if (s) spy.observe(s);
    });
  }

  // the deck ledger: the row you point at or Tab to is the selected one, and the navigator rides the
  // ratlines up to it one rung at a time, then back down to the foot of the mast
  const ledgerClimb = () => {
    const works = document.querySelector(".works");
    const climber = document.querySelector(".climber");
    if (!works || !climber) return;
    const rung = 8 * parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--px"));
    const hovers = matchMedia("(any-hover: hover)"); // a touch screen's last tap leaves :hover behind
    let row = null;
    let at = 0;
    const go = (li) => {
      if (li === row) return;
      row?.classList.remove("sel");
      row = li;
      li?.classList.add("sel");
      const dy = li ? li.offsetTop + li.offsetHeight / 2 - (climber.offsetTop + climber.offsetHeight / 2) : 0;
      climber.style.setProperty("--n", Math.max(1, Math.round(Math.abs(dy - at) / rung)));
      climber.style.setProperty("--dy", dy + "px");
      at = dy;
    };
    const focused = () => {
      const a = document.activeElement;
      return works.contains(a) && a.matches(":focus-visible") ? a.closest("li") : null;
    };
    works.addEventListener("pointerover", (e) => e.pointerType !== "touch" && go(e.target.closest("li")));
    works.addEventListener("pointerleave", () => go(focused()));
    works.addEventListener("focusin", (e) => e.target.matches(":focus-visible") && go(e.target.closest("li")));
    works.addEventListener(
      "focusout",
      (e) => works.contains(e.relatedTarget) || go(hovers.matches ? works.querySelector("li:hover") : null),
    );
  };
  ledgerClimb();

  // a neutral crewmate glances up when its room scrolls into view, then goes back to its work
  if (!reduce && "IntersectionObserver" in window) {
    const seen = new IntersectionObserver(
      (entries) =>
        entries.forEach((en) => {
          if (!en.isIntersecting) return;
          const el = en.target;
          el.classList.add("looking");
          setTimeout(() => el.classList.remove("looking"), 2200);
          seen.unobserve(el);
        }),
      { threshold: 0.6 },
    );
    document.querySelectorAll(".room, .nest").forEach((el) => seen.observe(el));
  }

  // ship's own keys, since this is the page about ship. alt-a calls the next crewmate who needs you (blocked,
  // then error, then unseen, the order the sidebar sorts them), and alt-q detaches. Alt and nothing else
  const shipKeys = () => {
    const crew = [".a-blocked", ".a-error", ".a-unseen"].map((s) => document.querySelector(s)?.closest("li"));
    let turn = 0;
    let called;
    let toast;
    let toastTimer;
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && toast) toast.hidden = true;
      if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || e.repeat || e.isComposing) return;
      if (e.target.closest?.("input, textarea, select, [contenteditable]")) return;
      if (e.code === "KeyA" && crew.every(Boolean)) {
        e.preventDefault();
        const li = crew[turn++ % crew.length];
        li.scrollIntoView({ block: "center" });
        crew.forEach((c) => c.classList.toggle("called", c === li));
        clearTimeout(called);
        called = setTimeout(() => li.classList.remove("called"), 1200);
        say(
          "jumped to the " + li.querySelector(".act").className.match(/a-(blocked|error|unseen)/)[1] + " crewmate",
          2000,
        );
      } else if (e.code === "KeyQ") {
        e.preventDefault();
        if (!toast) {
          toast = Object.assign(document.createElement("p"), { className: "term mount toast" });
          toast.setAttribute("aria-hidden", "true");
          toast.textContent = "detached. the server and everything in it keeps running. run ship to come back.";
          document.body.append(toast);
        }
        toast.hidden = false;
        toast.style.animation = "none";
        void toast.offsetWidth;
        toast.style.animation = "";
        say(toast.textContent, 5000);
        clearTimeout(toastTimer);
        toastTimer = setTimeout(() => (toast.hidden = true), 5000);
      }
    });
    console.info("alt-a and alt-q work on this page, the same as in ship.");
  };
  shipKeys();

  // the bottle at the waterline is an agent: unseen until you look at it, idle after, and a press sends it off
  // again. The tag always shows the glyph and the word, and the button's own words follow
  const bottleTag = () => {
    const b = document.querySelector(".bottle");
    if (!b) return;
    const glyph = b.querySelector(".g");
    const word = b.querySelector(".tag b");
    const words = b.querySelector(".sr-only");
    const unseen = words.textContent;
    b.addEventListener("click", () => {
      const idle = b.classList.toggle("seen");
      glyph.className = "g " + (idle ? "s-idle" : "s-unseen");
      glyph.textContent = idle ? "○" : "◆";
      word.textContent = idle ? "idle" : "unseen";
      words.textContent = idle ? "the bottle is idle now that you looked. press to send it off again." : unseen;
      say(idle ? "the bottle is idle" : "the bottle is unseen", 2000);
    });
  };
  bottleTag();

  // the neutral crewmates follow the pointer with their eyes: a sheet of nine faces, and the cell is picked
  // by which side of the face the pointer is on. With the keyboard they look at what has focus. Not on touch,
  // and not with reduced motion
  const eyesFollow = () => {
    const eyes = [...document.querySelectorAll(".eyes")];
    if (reduce || !eyes.length) return;
    let pointer = null;
    let focus = null;
    let queued = false;
    const aim = () => {
      queued = false;
      const f = focus?.getBoundingClientRect();
      const at = pointer || (f && { x: f.left + f.width / 2, y: f.top + f.height / 2 });
      if (!at) return eyes.forEach((el) => el.classList.remove("look"));
      eyes.forEach((el) => {
        const r = el.getBoundingClientRect();
        if (r.bottom < 0 || r.top > innerHeight) return el.classList.remove("look");
        const dx = at.x - (r.left + r.width / 2);
        const dy = at.y - (r.top + r.height / 2);
        const lx = dx < -60 ? 0 : dx > 60 ? 2 : 1;
        const ly = dy < -48 ? 0 : dy > 48 ? 2 : 1;
        el.style.setProperty("--lx", lx);
        el.style.setProperty("--ly", ly);
        el.classList.toggle("look", lx !== 1 || ly !== 1);
      });
    };
    const soon = () => queued || ((queued = true), requestAnimationFrame(aim));
    addEventListener(
      "pointermove",
      (e) => {
        if (e.pointerType === "touch") return;
        pointer = { x: e.clientX, y: e.clientY };
        focus = null;
        soon();
      },
      { passive: true },
    );
    addEventListener("scroll", soon, { passive: true });
    addEventListener("focusin", (e) => {
      focus = e.target.matches?.(":focus-visible") ? e.target : null;
      if (focus) pointer = null;
      soon();
    });
    addEventListener("focusout", () => {
      focus = null;
      soon();
    });
    document.documentElement.addEventListener("pointerleave", () => {
      pointer = null;
      soon();
    });
  };
  eyesFollow();

  // a loop that is off screen has nobody to look at it: pause it until it comes back
  if ("IntersectionObserver" in window) {
    const watch = new IntersectionObserver((entries) =>
      entries.forEach((en) => en.target.classList.toggle("off", !en.isIntersecting)),
    );
    document.querySelectorAll(".act").forEach((el) => watch.observe(el));
  }

  // the recording loops on its own while it's on screen; with reduced motion, or without this script, it
  // waits on its poster with controls
  if (!reduce && "IntersectionObserver" in window) {
    const reel = new IntersectionObserver((entries) =>
      entries.forEach((en) => {
        if (!en.isIntersecting) en.target.pause();
        else if (!en.target.dataset.held) en.target.play().catch(() => {});
      }),
    );
    document.querySelectorAll(".recording video").forEach((v) => {
      v.controls = false;
      reel.observe(v);
    });
  }

  // the recordings' own controls, in the chin of the bezel: pause or play, and how far along the loop is.
  // A pause is the visitor's, so scrolling away and back does not undo it (data-held)
  const reelControls = () => {
    if (reduce || !("IntersectionObserver" in window)) return;
    document.querySelectorAll(".recording .screen").forEach((screen) => {
      const v = screen.querySelector("video");
      if (!v) return;
      const btn = Object.assign(document.createElement("button"), { type: "button", className: "reel-btn" });
      const fill = document.createElement("i");
      const track = Object.assign(document.createElement("span"), { className: "reel-track", tabIndex: 0 });
      track.setAttribute("role", "slider");
      track.setAttribute("aria-label", "position in the recording");
      track.setAttribute("aria-valuemin", "0");
      track.append(fill);
      const bar = Object.assign(document.createElement("div"), { className: "reel" });
      bar.append(btn, track);
      screen.append(bar);
      screen.classList.add("has-reel");
      const sync = () => {
        btn.textContent = v.paused ? "play" : "pause";
        btn.setAttribute("aria-label", (v.paused ? "play" : "pause") + " the recording");
      };
      btn.addEventListener("click", () => {
        if (v.paused) {
          delete v.dataset.held;
          v.play().catch(() => {});
        } else {
          v.dataset.held = "1";
          v.pause();
        }
      });
      v.addEventListener("play", sync);
      v.addEventListener("pause", sync);
      const show = () => {
        if (!v.duration) return;
        bar.style.setProperty("--p", v.currentTime / v.duration);
        track.setAttribute("aria-valuemax", Math.round(v.duration));
        track.setAttribute("aria-valuenow", Math.round(v.currentTime));
        track.setAttribute("aria-valuetext", `${Math.round(v.currentTime)} of ${Math.round(v.duration)} seconds`);
      };
      v.addEventListener("timeupdate", show);
      v.addEventListener("loadedmetadata", show);
      // scrubbing: drag along the groove, or arrow keys a second at a time. The loop waits while you drag
      // and carries on afterwards, unless you had paused it
      const seek = (t) => {
        if (!v.duration) return;
        v.currentTime = Math.min(Math.max(t, 0), v.duration);
        show();
      };
      const at = (e) => {
        const r = track.getBoundingClientRect();
        return ((e.clientX - r.left) / r.width) * v.duration;
      };
      let resume = false;
      track.addEventListener("pointerdown", (e) => {
        if (e.button !== 0 || !v.duration) return;
        track.setPointerCapture(e.pointerId);
        resume = !v.paused;
        v.pause();
        if (e.pointerType !== "touch") seek(at(e)); // a touch may turn out to be a page scroll: wait for it
      });
      track.addEventListener("pointermove", (e) => track.hasPointerCapture(e.pointerId) && seek(at(e)));
      const done = (e) => {
        if (!track.hasPointerCapture(e.pointerId)) return;
        track.releasePointerCapture(e.pointerId);
        if (resume) v.play().catch(() => {});
      };
      track.addEventListener("pointerup", (e) => {
        if (track.hasPointerCapture(e.pointerId)) seek(at(e));
        done(e);
      });
      track.addEventListener("pointercancel", done);
      track.addEventListener("keydown", (e) => {
        const step = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[e.key];
        if (step) seek(v.currentTime + step);
        else if (e.key === "Home") seek(0);
        else if (e.key === "End") seek(v.duration - 0.05);
        else return;
        e.preventDefault();
      });
      sync();
    });
  };
  reelControls();
})();
