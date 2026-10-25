// Bellcart review interface: the five steps of the assistant.
//
// Every value from the scan (folder and file names) is personal data and
// may hold any character. It is only ever put in the page as text, through
// `h()`, never as HTML.
"use strict";

const token = location.hash.slice(1);
let S = {};          // interface strings, raw patterns
let session = {};    // what /api/session said
let review = null;   // what /api/review said
let skipped = new Set();
let step = "choose";
let settledOpen = false; // keep the settled list open across redraws
let scope = null;    // a parent chosen in the card's path: { card, folder }
let wanted = null;   // a card asked for from the summary, shown first

// ---------------------------------------------------------------- helpers

function t(key, args) {
  let text = S[key] ?? key;
  for (const [name, value] of Object.entries(args || {})) {
    text = text.split(`{ $${name} }`).join(String(value));
  }
  return text;
}

function plural(count, one, other, args) {
  // French puts 0 in the singular: 0 dossier.
  const single = count === 1 || (session.lang === "fr" && count === 0);
  // Shown with the language's digit grouping: 214 886, not 214886.
  return t(single ? one : other, { ...args, count: Number(count).toLocaleString(session.lang) });
}

function h(tag, attrs, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === false || v === null || v === undefined) continue;
    if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (k === "class") el.className = v;
    else el.setAttribute(k, v === true ? "" : v);
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    el.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return el;
}

function bytes(n) {
  // French writes octets: o, Ko, Mo, Go.
  const units = session.lang === "fr" ? ["o", "Ko", "Mo", "Go", "To"] : ["B", "KB", "MB", "GB", "TB"];
  let v = n, u = 0;
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u++; }
  const text = u === 0 ? String(n) : v.toFixed(1);
  return `${session.lang === "fr" ? text.replace(".", ",") : text} ${units[u]}`;
}

function month(secs) {
  if (!secs) return "";
  const d = new Date(secs * 1000);
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, "0")}`;
}

function shown(path) { return path === "" ? t("ui-root") : path; }

// Same test as the engine: `path` is `folder` or lies under it.
function within(path, folder) {
  return folder === "" || path === folder || path.startsWith(`${folder}/`);
}

async function api(method, path, body) {
  const response = await fetch(path, {
    method,
    headers: { "X-Bellcart-Token": token, "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || response.statusText);
  return data;
}

function toast(text) {
  const el = document.getElementById("toast");
  el.textContent = text;
  el.hidden = false;
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => { el.hidden = true; }, 2500);
}

function show(...children) {
  const main = document.getElementById("main");
  // Parts that do not apply come as null: leave them out.
  main.replaceChildren(...children.filter((c) => c !== null && c !== undefined && c !== false));
  renderSteps();
}

function failure(error) {
  return h("p", { class: "error", role: "alert" }, t("ui-error", { error: error.message || error }));
}

// ---------------------------------------------------------------- steps

const STEPS = ["choose", "analyse", "review", "summary", "export"];

function renderSteps() {
  const list = document.getElementById("steps");
  const at = STEPS.indexOf(step);
  list.replaceChildren(...STEPS.map((name, i) => h("li", {
    class: i < at ? "done" : null,
    "aria-current": i === at ? "step" : null,
  }, h("span", { class: "n" }, i + 1), t(`ui-step-${name}`))));
}

// 1. Choose

function choose(message) {
  step = "choose";
  const input = h("input", { type: "text", id: "profile", value: session.profile, spellcheck: "false", autocomplete: "off" });
  const saved = session.saved;
  const scan = (resume) => async (event) => {
    event.preventDefault();
    try {
      await api("POST", "/api/scan", { profile: input.value, resume });
      analyse();
    } catch (error) {
      choose(error);
    }
  };
  // Decisions saved by an earlier session on this profile: resume them, or
  // start over. They only apply to the profile they were made on.
  const resumeView = saved ? h("section", { class: "panel stack", "aria-labelledby": "resume-title" },
    h("h2", { id: "resume-title" }, t("ui-resume-title")),
    h("p", { class: "muted small" }, plural(saved.count, "ui-resume-help-one", "ui-resume-help-other", { profile: saved.profile, file: saved.file })),
    h("div", { class: "row" },
      h("button", { class: "primary", type: "button", onclick: (e) => { input.value = saved.profile; scan(true)(e); } }, t("ui-resume")),
      h("button", { type: "button", onclick: scan(false) }, t("ui-start-over")))) : null;
  show(
    h("h1", {}, t("ui-choose-title")),
    h("p", { class: "muted" }, t("ui-choose-help")),
    resumeView,
    // Enter resumes when the field still names the profile with saved
    // decisions: starting over must be the explicit button.
    h("form", { class: "panel stack", onsubmit: (e) => scan(Boolean(saved) && input.value.trim() === saved.profile)(e) },
      h("div", {}, h("label", { for: "profile" }, t("ui-choose-label")), input),
      message ? failure(message) : null,
      h("div", { class: "row" }, h("button", { class: saved ? null : "primary", type: "submit" }, t("ui-choose-start")))),
  );
  input.focus();
}

// 2. Analyse

function analyse() {
  step = "analyse";
  const counts = h("p", { class: "progress", "aria-live": "polite" }, "");
  show(
    h("h1", {}, t("ui-scanning-title")),
    h("p", { class: "muted" }, t("ui-scanning-help")),
    h("div", { class: "panel row" }, h("div", { class: "spinner", "aria-hidden": "true" }), counts),
  );
  const poll = async () => {
    try {
      const p = await api("GET", "/api/progress");
      counts.textContent = t("scan-progress", { folders: p.folders.toLocaleString(session.lang), size: bytes(p.bytes) });
      if (p.phase === "review") return startReview();
      if (p.phase === "failed") return choose(new Error(t("ui-failed", { error: p.error })));
      setTimeout(poll, 300);
    } catch (error) {
      show(failure(error));
    }
  };
  poll();
}

// 3. Review

async function startReview() {
  skipped = new Set();
  review = await api("GET", "/api/review");
  step = "review";
  renderReview();
  if (review.resumed) toast(plural(review.resumed, "ui-resumed-one", "ui-resumed-other"));
}

function carts() {
  const queueBytes = review.queue.reduce((sum, c) => sum + c.bytes, 0);
  const cart = (decision, label, size, count) => h("div", { class: `cart c-${decision}` },
    h("div", { class: "label" }, label),
    h("div", { class: "size" }, size),
    h("div", { class: "count" }, count));
  const tot = (d) => review.totals[d] || { bytes: 0, files: 0 };
  return h("section", { class: "carts", "aria-label": t("ui-carts") },
    cart("migrate", t("decision-migrate"), bytes(tot("migrate").bytes), plural(tot("migrate").files, "ui-file-one", "ui-file-other")),
    cart("archive", t("decision-archive"), bytes(tot("archive").bytes), plural(tot("archive").files, "ui-file-one", "ui-file-other")),
    cart("leave", t("decision-leave"), bytes(tot("leave").bytes), plural(tot("leave").files, "ui-file-one", "ui-file-other")),
    cart("review", t("ui-to-decide"), bytes(queueBytes), plural(review.queue.length, "ui-folder-one", "ui-folder-other")));
}

// A short reason reads as a tag; a sentence reads as a sentence.
function reasonView(reason) {
  return reason.length <= 60
    ? h("p", {}, h("span", { class: "chip" }, reason))
    : h("p", { class: "reason" }, reason);
}

// A path inside the card's folder, shown from that folder.
function inside(path, folder) {
  return folder !== "" && path.startsWith(`${folder}/`) ? path.slice(folder.length + 1) : path;
}

function nextCard() {
  return review.queue.find((c) => c.path === wanted)
    || review.queue.find((c) => !skipped.has(c.path)) || review.queue[0];
}

// The folder a decision on this card applies to, and whether it is a parent.
function decisionFor(card) {
  return scope && scope.card === card.path ? { path: scope.folder, below: true } : { path: card.path, below: false };
}

// The card's path, each parent a button that moves the decision up to it.
function crumbs(card) {
  if (card.path === "") return shown(card.path);
  const parts = card.path.split("/");
  const out = [];
  parts.forEach((part, i) => {
    const folder = parts.slice(0, i + 1).join("/");
    if (i > 0) out.push(h("span", { class: "sep", "aria-hidden": "true" }, "/"));
    const chosen = scope && scope.card === card.path && scope.folder === folder;
    if (i === parts.length - 1) {
      out.push(scope && scope.card === card.path
        ? h("button", { class: "crumb", type: "button", onclick: () => { scope = null; renderReview(); } }, part)
        : h("span", { class: "here" }, part));
    } else {
      out.push(h("button", {
        class: chosen ? "crumb chosen" : "crumb",
        type: "button",
        title: t("ui-crumb-up", { path: folder }),
        "aria-pressed": chosen ? "true" : "false",
        onclick: () => { scope = { card: card.path, folder }; renderReview(); },
      }, part));
    }
  });
  return out;
}

// What a decision on a parent covers, from what the page already knows.
function scopeView(card) {
  if (!scope || scope.card !== card.path) return null;
  const folder = scope.folder;
  const pending = review.queue.filter((c) => within(c.path, folder));
  const size = bytes(pending.reduce((sum, c) => sum + c.bytes, 0));
  const below = review.settled.filter((n) => within(n.path, folder));
  const kept = below.filter((n) => n.by === "rule" || n.by === "recipe").length;
  const changed = below.length - kept;
  return h("section", { class: "scope", "aria-live": "polite" },
    h("h2", {}, t("ui-scope-title", { path: folder })),
    h("ul", { class: "small" },
      h("li", {}, plural(pending.length, "ui-scope-pending-one", "ui-scope-pending-other", { size })),
      changed ? h("li", {}, plural(changed, "ui-scope-changed-one", "ui-scope-changed-other")) : null,
      kept ? h("li", {}, plural(kept, "ui-scope-kept-one", "ui-scope-kept-other")) : null),
    h("button", { class: "link", type: "button", onclick: () => { scope = null; renderReview(); } },
      t("ui-scope-back", { path: card.path })));
}

function listOf(entries, total) {
  const items = entries.map((e) => h("li", {}, h("span", { title: e.name }, e.name), h("span", {}, bytes(e.bytes))));
  if (total > entries.length) items.push(h("li", { class: "muted" }, h("span", {}, t("ui-more", { count: total - entries.length })), h("span", {}, "")));
  return h("ul", { class: "list" }, items);
}

function cardView(card) {
  const index = review.queue.indexOf(card) + 1;
  const kinds = card.extensions.map((e) => {
    // Rounded down, as in signatures: 99.7 % reads 99 %, never 100 %.
    const share = card.bytes ? Math.floor((e.bytes * 100) / card.bytes) : 0;
    return h("li", {},
      h("span", {}, e.ext === "" ? t("ui-no-extension") : `.${e.ext}`),
      h("span", {}, `${share < 1 && e.bytes > 0 ? "<1" : share} %`));
  });
  let opinion = null;
  if (card.opinion === "leave") {
    opinion = h("p", { class: "opinion", style: null }, t("ui-card-opinion-leave"));
  } else if (card.opinion) {
    opinion = h("p", { class: "opinion" }, t("ui-card-opinion", { decision: t(`decision-${card.opinion}`) }));
  }
  const decide = (decision) => () => { const at = decisionFor(card); setDecision(at.path, decision, at.below); };
  return h("article", { class: "panel card stack", "aria-labelledby": "card-path", tabindex: "-1" },
    h("div", {},
      h("div", { class: "muted small" }, t("ui-card-position", { index, count: review.queue.length })),
      card.app ? h("h2", { class: "app" }, card.app) : null,
      // The card's title: the application, or else the folder itself.
      h(card.app ? "div" : "h2", { class: "path", id: "card-path" }, crumbs(card)),
      h("div", { class: "facts" },
        h("span", {}, bytes(card.bytes)),
        h("span", {}, plural(card.files, "ui-file-one", "ui-file-other")),
        card.oldest ? h("span", {}, t("ui-card-modified", { oldest: month(card.oldest), newest: month(card.newest) })) : null,
        card.unreadable ? h("span", { class: "error" }, plural(card.unreadable, "ui-unreadable-one", "ui-unreadable-other")) : null)),
    scopeView(card),
    // A proposal from the pack: explained, and taken only by the user.
    card.propose ? h("p", { class: `opinion c-${card.propose}` },
      h("strong", {}, t("ui-card-propose", { decision: t(`decision-${card.propose}`) })), " ", card.reason)
      : card.reason ? reasonView(card.reason) : null,
    opinion,
    h("div", { class: "columns" },
      h("section", {}, h("h3", {}, t("ui-card-kinds")), h("ul", { class: "list" }, kinds)),
      h("section", {}, h("h3", {}, t("ui-card-folders")), card.folders.length ? listOf(card.folders, card.folder_count) : h("p", { class: "muted small" }, t("ui-none"))),
      h("section", {}, h("h3", {}, t("ui-card-files")), card.sample_files.length ? listOf(card.sample_files, card.file_count) : h("p", { class: "muted small" }, t("ui-none")))),
    card.settled.length ? h("section", {},
      h("h3", { class: "muted small" }, t("ui-card-settled")),
      h("ul", { class: "list" }, card.settled.map((s) => h("li", {},
        h("span", { title: s.path }, inside(s.path, card.path)),
        h("span", {}, `${t(`decision-${s.decision}`)} · ${bytes(s.bytes)}`))))) : null,
    h("div", { class: "actions" },
      ["migrate", "archive", "leave"].map((d) => h("button", {
        class: card.propose === d ? `d d-${d} proposed` : `d d-${d}`,
        onclick: decide(d),
      }, t(`ui-action-${d}`), h("kbd", {}, card.propose === d ? t("ui-key-enter") : d[0].toUpperCase()))),
      h("span", { class: "spacer" }),
      h("button", { onclick: () => { skipped.add(card.path); scope = null; wanted = null; renderReview(); } }, t("ui-action-later"), h("kbd", {}, "→"))));
}

function settledView() {
  const rows = review.settled.map((n) => {
    const select = h("select", {
      "aria-label": t("ui-change-for", { path: shown(n.path) }),
      "data-path": n.path,
      onchange: (e) => setDecision(n.path, e.target.value),
    }, ["migrate", "archive", "leave", "review"].map((d) => h("option", { value: d, selected: d === n.decision }, t(`decision-${d}`))));
    return h("tr", {},
      h("td", {}, h("span", { class: `pill c-${n.decision}`, style: null }, t(`decision-${n.decision}`))),
      h("td", { class: "path" }, shown(n.path)),
      h("td", { class: "num" }, bytes(n.bytes)),
      h("td", { class: "muted" }, n.why),
      h("td", {}, select));
  });
  return h("details", {
    class: "settled panel",
    open: settledOpen,
    ontoggle: (e) => { settledOpen = e.target.open; },
  },
    h("summary", {}, t("ui-settled-title", { count: review.settled.length })),
    h("p", { class: "muted small" }, t("ui-settled-help")),
    h("div", { class: "table-wrap" }, h("table", {},
      h("thead", {}, h("tr", {}, h("th", {}, t("ui-col-decision")), h("th", {}, t("ui-col-folder")), h("th", {}, t("ui-col-size")), h("th", {}, t("ui-col-why")), h("th", {}, t("ui-col-change")))),
      h("tbody", {}, rows))));
}

function arbiterView() {
  if (!session.arbiter || review.queue.length === 0) return null;
  return h("div", { class: "row" },
    h("button", { onclick: askArbiter }, t("ui-arbiter-ask")),
    h("span", { class: "muted small" }, t("ui-arbiter-help", { url: session.arbiter })));
}

function renderReview() {
  const card = review.queue.length ? nextCard() : null;
  const top = h("div", { class: "row" },
    h("h1", { style: null }, t("ui-review-title")),
    h("span", { class: "spacer" }));
  show(
    top,
    h("p", { class: "muted" }, t("ui-review-help")),
    h("p", { class: "muted small" }, review.saved_to ? t("ui-saved-to", { file: review.saved_to }) : t("ui-not-saved")),
    review.applications.length ? h("p", { class: "small" }, t("ui-applications", { names: review.applications.join(", ") })) : null,
    carts(),
    arbiterView(),
    card ? cardView(card) : h("section", { class: "panel stack" },
      h("h2", {}, t("ui-queue-empty")),
      h("p", { class: "muted" }, t("ui-queue-empty-help"))),
    h("div", { class: "row", style: null },
      h("button", { class: card ? null : "primary", onclick: summaryStep }, t("ui-go-export"))),
    settledView(),
  );
}

async function setDecision(path, decision, below) {
  // A change made in the settled list keeps the reader there.
  const fromList = Boolean(document.activeElement?.closest("details.settled"));
  try {
    const savedTo = review.saved_to;
    review = await api("POST", "/api/decide", { path, decision, below: below === true });
    review.saved_to = savedTo;
    if (review.save_error) {
      toast(t("ui-save-failed", { error: review.save_error }));
      skipped.delete(path);
      scope = null;
      return renderReview();
    }
    skipped.delete(path);
    scope = null;
    wanted = null;
    const named = { path: shown(path), decision: t(`decision-${decision}`) };
    toast(below
      ? plural(review.changed, "ui-decided-below-one", "ui-decided-below-other", named)
      : t("ui-decided", named));
    renderReview();
    // Keep the keyboard on the card that comes next, not on the page.
    if (!fromList) document.querySelector("article.card")?.focus({ preventScroll: true });
  } catch (error) {
    toast(t("ui-error", { error: error.message }));
  }
}

document.addEventListener("keydown", (event) => {
  if (step !== "review" || event.ctrlKey || event.metaKey || event.altKey) return;
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || document.getElementById("dialog").open) return;
  const card = review && review.queue.length ? nextCard() : null;
  if (!card) return;
  const key = event.key.toLowerCase();
  const decisions = { m: "migrate", a: "archive", l: "leave" };
  // Enter takes the proposal, for the card's own folder only.
  if (event.key === "Enter" && card.propose && !(scope && scope.card === card.path)) {
    if (target instanceof HTMLButtonElement) return;
    event.preventDefault();
    return setDecision(card.path, card.propose);
  }
  if (decisions[key]) { event.preventDefault(); const at = decisionFor(card); setDecision(at.path, decisions[key], at.below); }
  if (event.key === "ArrowRight") { event.preventDefault(); skipped.add(card.path); scope = null; wanted = null; renderReview(); }
  if (event.key === "Escape" && scope) { event.preventDefault(); scope = null; renderReview(); }
});

// The arbiter: show exactly what goes out, send only on a click.

async function askArbiter() {
  const dialog = document.getElementById("dialog");
  try {
    const preview = await api("GET", "/api/arbiter");
    if (!preview.count) return;
    const privacy = t(preview.privacy === "full" ? "ui-privacy-full" : "ui-privacy-redacted");
    dialog.replaceChildren(
      h("h2", {}, t("ui-arbiter-preview-title", { count: preview.count })),
      h("p", { class: "muted small" }, t("ui-arbiter-preview-help", { url: preview.url, model: preview.model, privacy })),
      h("pre", {}, preview.request),
      h("div", { class: "row" },
        h("button", { class: "primary", onclick: async () => { dialog.close(); await runArbiter(); } }, t("ui-arbiter-send")),
        h("button", { onclick: () => dialog.close() }, t("ui-cancel"))));
    dialog.showModal();
  } catch (error) {
    toast(t("ui-error", { error: error.message }));
  }
}

async function runArbiter() {
  try {
    await api("POST", "/api/arbiter");
  } catch (error) {
    return toast(t("ui-error", { error: error.message }));
  }
  const poll = async () => {
    const p = await api("GET", "/api/progress");
    const a = p.arbiter;
    toast(t("ui-arbiter-progress", { done: a.done, total: a.total }));
    if (a.running) return setTimeout(poll, 500);
    review = await api("GET", "/api/review");
    renderReview();
    if (a.error) toast(t("ui-arbiter-failed", { error: a.error }));
  };
  poll();
}

// 4. Summary: what the plan will say, read before anything is written.

// Folders whose content is the user's own, by their name on disk (which
// Windows does not translate).
const PERSONAL = ["desktop", "documents", "pictures", "videos", "music"];

function personal(path) {
  // In a scan of a whole drive, the personal folders are in the profile.
  const inside = review.profile ? path.slice(review.profile.length + 1) : path;
  if (review.profile && !path.toLowerCase().startsWith(`${review.profile.toLowerCase()}/`)) return false;
  return PERSONAL.includes(inside.split("/")[0].toLowerCase());
}

function backToCard(path) {
  wanted = path;
  scope = null;
  step = "review";
  renderReview();
  document.querySelector("article.card")?.focus({ preventScroll: true });
}

function backToSettled(path) {
  settledOpen = true;
  step = "review";
  renderReview();
  const select = [...document.querySelectorAll("details.settled select")].find((s) => s.dataset.path === path);
  select?.scrollIntoView({ block: "center" });
  select?.focus({ preventScroll: true });
}

// The carts as one bar, each part as wide as its share of the bytes.
function splitView() {
  const parts = ["migrate", "archive", "leave", "review"].map((d) => {
    const size = d === "review" ? review.queue.reduce((s, c) => s + c.bytes, 0) : (review.totals[d] || { bytes: 0 }).bytes;
    return { d, size, label: d === "review" ? t("ui-to-decide") : t(`decision-${d}`) };
  });
  const total = parts.reduce((s, p) => s + p.size, 0) || 1;
  const share = (p) => Math.round((p.size * 100) / total);
  const described = parts.map((p) => t("ui-summary-share", { decision: p.label, size: bytes(p.size), percent: share(p) })).join(", ");
  return h("section", { class: "panel stack", "aria-labelledby": "split-title" },
    h("h2", { id: "split-title" }, t("ui-summary-split")),
    h("div", { class: "split", role: "img", "aria-label": described },
      parts.filter((p) => p.size > 0).map((p) => {
        // Set through the DOM: the page's policy refuses style attributes.
        const part = h("i", { class: `c-${p.d}` });
        part.style.flexGrow = String(p.size / total);
        return part;
      })),
    h("ul", { class: "legend small" }, parts.map((p) => h("li", { class: `c-${p.d}` },
      h("span", { class: "swatch", "aria-hidden": "true" }), h("span", {}, p.label),
      h("strong", {}, bytes(p.size)), h("span", { class: "muted" }, `${share(p)} %`)))));
}

function rowLabel(row) { return row.app ? `${row.app} · ${shown(row.path)}` : shown(row.path); }

// One point to check: a sentence, then the folders it is about, each with
// the way back to where it is decided.
function attention(kind, text, rows) {
  const first = rows.slice(0, 8);
  return h("li", { class: `attention ${kind}` },
    h("p", {}, text),
    first.length ? h("ul", { class: "list" }, first.map((r) => h("li", {},
      h("span", { title: r.path }, rowLabel(r)),
      h("span", {}, bytes(r.bytes), " ", r.go)))) : null,
    rows.length > first.length ? h("p", { class: "muted small" }, t("ui-more", { count: rows.length - first.length })) : null);
}

function attentionView() {
  const sum = (rows) => bytes(rows.reduce((s, r) => s + r.bytes, 0));
  const decide = (c) => ({ ...c, go: h("button", { class: "link", type: "button", onclick: () => backToCard(c.path) }, t("ui-summary-decide")) });
  const change = (n) => ({ ...n, go: h("button", { class: "link", type: "button", onclick: () => backToSettled(n.path) }, t("ui-summary-change")) });
  const chosen = (n) => n.by === "user" || n.by === "arbiter";
  const points = [];
  const pending = review.queue.map(decide);
  if (pending.length) {
    points.push(attention("warn", plural(pending.length, "ui-summary-undecided-one", "ui-summary-undecided-other", { size: sum(pending) }), pending));
  }
  // A wrong leave costs data: what a person or the arbiter chose to leave
  // comes first, then what rules left inside personal folders.
  const leftByChoice = review.settled.filter((n) => n.decision === "leave" && chosen(n)).map(change);
  if (leftByChoice.length) {
    points.push(attention("warn", plural(leftByChoice.length, "ui-summary-left-by-you-one", "ui-summary-left-by-you-other", { size: sum(leftByChoice) }), leftByChoice));
  }
  const leftInPersonal = review.settled.filter((n) => n.decision === "leave" && !chosen(n) && personal(n.path) && n.bytes > 0).map(change);
  if (leftInPersonal.length) {
    points.push(attention("info", plural(leftInPersonal.length, "ui-summary-left-personal-one", "ui-summary-left-personal-other", { size: sum(leftInPersonal) }), leftInPersonal));
  }
  const undecidedPrograms = (review.programs || []).filter((p) => p.reinstall === "undecided");
  if (undecidedPrograms.length) {
    points.push(attention("info", plural(undecidedPrograms.length, "ui-summary-programs-undecided-one", "ui-summary-programs-undecided-other"), []));
  }
  if (review.unreadable) {
    points.push(attention("warn", plural(review.unreadable, "ui-summary-unreadable-one", "ui-summary-unreadable-other"), []));
  }
  return h("section", { id: "attention", class: "panel stack", "aria-labelledby": "attention-title" },
    h("h2", { id: "attention-title" }, t("ui-summary-attention")),
    points.length ? h("ul", { class: "points" }, points) : h("p", { class: "ok" }, t("ui-summary-all-clear")));
}

// The largest folders of each cart, with why they are there.
function largestView() {
  const column = (d) => {
    const rows = review.settled.filter((n) => n.decision === d && n.bytes > 0).slice(0, 5);
    return h("section", { class: `cart-top c-${d}` },
      h("h3", {}, t(`decision-${d}`), " ", h("span", { class: "muted" }, bytes((review.totals[d] || { bytes: 0 }).bytes))),
      rows.length ? h("ul", { class: "list tall" }, rows.map((n) => h("li", {},
        h("span", { class: "two-lines" },
          h("span", { class: "path", title: n.path }, shown(n.path)),
          h("span", { class: "muted small" }, n.why)),
        h("span", {}, bytes(n.bytes), " ",
          h("button", { class: "link", type: "button", onclick: () => backToSettled(n.path) }, t("ui-summary-change"))))))
        : h("p", { class: "muted small" }, t("ui-summary-cart-empty")));
  };
  return h("section", { class: "panel stack", "aria-labelledby": "largest-title" },
    h("h2", { id: "largest-title" }, t("ui-summary-largest")),
    h("div", { class: "columns" }, ["migrate", "archive", "leave"].map(column)));
}

// Programs found installed: whether each one is wanted on the new PC.
// The choice changes no folder decision, so only the points are redrawn.
function programsView() {
  const programs = [...(review.programs || [])].sort((a, b) => a.name.localeCompare(b.name, session.lang, { sensitivity: "base" }));
  if (!programs.length) return null;
  const choices = ["yes", "no", "undecided"];
  return h("section", { class: "panel stack", "aria-labelledby": "programs-title" },
    h("h2", { id: "programs-title" }, t("ui-summary-programs")),
    h("p", { class: "muted small" }, t("ui-summary-programs-help")),
    h("ul", { class: "list programs" }, programs.map((p) => h("li", {},
      h("span", { title: p.path }, p.name),
      h("select", {
        "aria-label": t("ui-reinstall-for", { name: p.name }),
        "data-path": p.path,
        onchange: (e) => setReinstall(p.path, e.target.value),
      }, choices.map((c) => h("option", { value: c, selected: c === p.reinstall }, t(`ui-reinstall-${c}`))))))));
}

async function setReinstall(path, reinstall) {
  try {
    const savedTo = review.saved_to;
    review = await api("POST", "/api/reinstall", { path, reinstall });
    review.saved_to = savedTo;
    if (review.save_error) toast(t("ui-save-failed", { error: review.save_error }));
    document.getElementById("attention")?.replaceWith(attentionView());
  } catch (error) {
    toast(t("ui-error", { error: error.message }));
  }
}

function summaryStep() {
  step = "summary";
  scope = null;
  wanted = null;
  show(
    h("h1", {}, t("ui-summary-title")),
    h("p", { class: "muted" }, t("ui-summary-help")),
    carts(),
    splitView(),
    attentionView(),
    largestView(),
    programsView(),
    review.applications.length ? h("p", { class: "small" }, t("ui-summary-applications", { names: review.applications.join(", ") })) : null,
    h("div", { class: "row" },
      h("button", { class: "primary", type: "button", onclick: exportStep }, t("ui-summary-confirm")),
      h("button", { type: "button", onclick: () => { step = "review"; renderReview(); } }, t("ui-summary-back"))),
  );
  window.scrollTo(0, 0);
}

// 5. Export

function exportStep(_, result, message) {
  step = "export";
  const input = h("input", { type: "text", id: "folder", value: result?.folder || session.export, spellcheck: "false", autocomplete: "off" });
  const write = async (event) => {
    event.preventDefault();
    try {
      const done = await api("POST", "/api/export", { folder: input.value });
      exportStep(null, { folder: input.value, files: done.files });
    } catch (error) {
      exportStep(null, { folder: input.value }, error);
    }
  };
  show(
    h("h1", {}, t("ui-export-title")),
    h("p", { class: "muted" }, t("ui-export-help")),
    h("form", { class: "panel stack", onsubmit: write },
      h("div", {}, h("label", { for: "folder" }, t("ui-export-label")), input),
      message ? failure(message) : null,
      h("div", { class: "row" },
        h("button", { class: "primary", type: "submit" }, t("ui-export-go")),
        h("button", { type: "button", onclick: summaryStep }, t("ui-back")))),
    result?.files ? h("section", { class: "panel stack", "aria-live": "polite" },
      h("h2", {}, t("ui-export-done")),
      h("ul", { class: "files" }, result.files.map((f) => h("li", {}, f))),
      h("p", {}, t("ui-export-next")),
      h("div", { class: "row" }, h("button", { onclick: quit }, t("ui-quit")))) : null,
  );
}

async function quit() {
  try { await api("POST", "/api/quit"); } catch (_) { /* already closed */ }
  step = "export";
  show(h("h1", {}, t("ui-closed")));
}

// ---------------------------------------------------------------- start

(async function start() {
  try {
    session = await api("GET", "/api/session");
    S = session.strings;
    document.documentElement.lang = session.lang;
    document.title = t("ui-title");
    if (session.phase === "scanning") return analyse();
    if (session.phase === "review") return startReview();
    choose();
  } catch (error) {
    document.getElementById("main").replaceChildren(failure(error));
  }
})();
