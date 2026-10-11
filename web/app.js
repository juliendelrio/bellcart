// Bellcart review interface: the four steps of the assistant.
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

// ---------------------------------------------------------------- helpers

function t(key, args) {
  let text = S[key] ?? key;
  for (const [name, value] of Object.entries(args || {})) {
    text = text.split(`{ $${name} }`).join(String(value));
  }
  return text;
}

function plural(count, one, other) {
  // French puts 0 in the singular: 0 dossier.
  const single = count === 1 || (session.lang === "fr" && count === 0);
  return t(single ? one : other, { count });
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

const STEPS = ["choose", "analyse", "review", "export"];

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
  const start = async (event) => {
    event.preventDefault();
    try {
      await api("POST", "/api/scan", { profile: input.value });
      analyse();
    } catch (error) {
      choose(error);
    }
  };
  show(
    h("h1", {}, t("ui-choose-title")),
    h("p", { class: "muted" }, t("ui-choose-help")),
    h("form", { class: "panel stack", onsubmit: start },
      h("div", {}, h("label", { for: "profile" }, t("ui-choose-label")), input),
      message ? failure(message) : null,
      h("div", { class: "row" }, h("button", { class: "primary", type: "submit" }, t("ui-choose-start")))),
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

function nextCard() {
  return review.queue.find((c) => !skipped.has(c.path)) || review.queue[0];
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
  const decide = (decision) => () => setDecision(card.path, decision);
  return h("article", { class: "panel card stack", "aria-labelledby": "card-path" },
    h("div", {},
      h("div", { class: "muted small" }, t("ui-card-position", { index, count: review.queue.length })),
      h("div", { class: "path", id: "card-path" }, shown(card.path)),
      h("div", { class: "facts" },
        h("span", {}, bytes(card.bytes)),
        h("span", {}, plural(card.files, "ui-file-one", "ui-file-other")),
        card.oldest ? h("span", {}, t("ui-card-modified", { oldest: month(card.oldest), newest: month(card.newest) })) : null,
        card.unreadable ? h("span", { class: "error" }, plural(card.unreadable, "ui-unreadable-one", "ui-unreadable-other")) : null)),
    card.reason ? h("p", {}, h("span", { class: "chip" }, card.reason)) : null,
    opinion,
    h("div", { class: "columns" },
      h("section", {}, h("h3", {}, t("ui-card-kinds")), h("ul", { class: "list" }, kinds)),
      h("section", {}, h("h3", {}, t("ui-card-folders")), card.folders.length ? listOf(card.folders, card.folder_count) : h("p", { class: "muted small" }, t("ui-none"))),
      h("section", {}, h("h3", {}, t("ui-card-files")), card.sample_files.length ? listOf(card.sample_files, card.file_count) : h("p", { class: "muted small" }, t("ui-none")))),
    card.settled.length ? h("section", {},
      h("h3", { class: "muted small" }, t("ui-card-settled")),
      h("ul", { class: "list" }, card.settled.map((s) => h("li", {},
        h("span", { title: s.path }, s.path),
        h("span", {}, `${t(`decision-${s.decision}`)} · ${bytes(s.bytes)}`))))) : null,
    h("div", { class: "actions" },
      h("button", { class: "d d-migrate", onclick: decide("migrate") }, t("ui-action-migrate"), h("kbd", {}, "M")),
      h("button", { class: "d d-archive", onclick: decide("archive") }, t("ui-action-archive"), h("kbd", {}, "A")),
      h("button", { class: "d d-leave", onclick: decide("leave") }, t("ui-action-leave"), h("kbd", {}, "L")),
      h("span", { class: "spacer" }),
      h("button", { onclick: () => { skipped.add(card.path); renderReview(); } }, t("ui-action-later"), h("kbd", {}, "→"))));
}

function settledView() {
  const rows = review.settled.map((n) => {
    const select = h("select", {
      "aria-label": t("ui-change-for", { path: shown(n.path) }),
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
    h("div", { style: null }, h("table", {},
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
    review.applications.length ? h("p", { class: "small" }, t("ui-applications", { names: review.applications.join(", ") })) : null,
    carts(),
    arbiterView(),
    card ? cardView(card) : h("section", { class: "panel stack" },
      h("h2", {}, t("ui-queue-empty")),
      h("p", { class: "muted" }, t("ui-queue-empty-help"))),
    h("div", { class: "row", style: null },
      h("button", { class: card ? null : "primary", onclick: exportStep }, t("ui-go-export"))),
    settledView(),
  );
}

async function setDecision(path, decision) {
  try {
    review = await api("POST", "/api/decide", { path, decision });
    skipped.delete(path);
    toast(t("ui-decided", { path: shown(path), decision: t(`decision-${decision}`) }));
    renderReview();
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
  if (decisions[key]) { event.preventDefault(); setDecision(card.path, decisions[key]); }
  if (event.key === "ArrowRight") { event.preventDefault(); skipped.add(card.path); renderReview(); }
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

// 4. Export

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
        h("button", { type: "button", onclick: () => { step = "review"; renderReview(); } }, t("ui-back")))),
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
