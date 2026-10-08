(() => {
  "use strict";
  const apiBase = (window.FORGE_API_BASE || "http://127.0.0.1:8765").replace(/\/$/, "");
  const endpoint = (path) => `${apiBase}${path}`;
  const page = document.body.dataset.page;
  const request = async (path, options = {}) => {
    const response = await fetch(endpoint(path), { credentials: "include", ...options });
    const body = response.status === 204 ? {} : await response.json().catch(() => ({}));
    if (!response.ok) throw Object.assign(new Error(body.error?.message || "Forge could not complete the request."), { status: response.status });
    return body;
  };
  // Lower-level call that returns the status and parsed body instead of
  // throwing, so the workbench can render an honest state for a refused
  // (409) apply — including the refreshed plan — rather than only an error.
  const requestStatus = async (path, options = {}) => {
    const response = await fetch(endpoint(path), { credentials: "include", ...options });
    const body = response.status === 204 ? {} : await response.json().catch(() => ({}));
    return { status: response.status, ok: response.ok, body };
  };
  const session = () => request("/v1/admin/session", { headers: { Accept: "application/json" } });

  // Slice 2: programmatic smooth scroll honors reduced motion. The CSS
  // `prefers-reduced-motion` guard cannot override an explicit JS
  // `{ behavior: "smooth" }` option, so the check lives here at the call
  // site. Missing targets never throw; absent `matchMedia` keeps "smooth".
  function scrollIntoViewRespectingMotion(target, options) {
    if (!target || typeof target.scrollIntoView !== "function") return;
    const reduce = typeof window.matchMedia === "function"
      && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    target.scrollIntoView({ block: (options && options.block) || "start", behavior: reduce ? "auto" : "smooth" });
  }

  // ---- Dashboard routing -------------------------------------------------
  //
  // Every sidebar destination is a real, deep-linkable path served as the one
  // application shell (see `asset_name` in src/web.rs). The router maps the
  // current pathname to exactly one view, shows only that view, and keeps the
  // sidebar active state, the topbar breadcrumb and the document title in
  // step. The anchors keep real `href`s, so a same-origin hard navigation to a
  // route still lands on the right view; click interception only upgrades it
  // to a pushState swap. `index.html` carries `<base href="/">` so its
  // relative assets resolve from the root even on a sub-path.
  const VIEW_BY_PATH = {
    "/": "projects",
    "/index.html": "projects",
    "/projects": "projects",
    "/workbench": "workbench",
    "/management": "management",
    "/portfolio": "portfolio",
    "/delivery": "delivery",
  };
  const VIEW_IDS = {
    projects: "view-projects",
    workbench: "workbench",
    management: "management",
    portfolio: "portfolio",
    delivery: "delivery",
  };
  const VIEW_CRUMBS = {
    projects: { crumb: "All projects", title: "Projects · Forge" },
    workbench: { crumb: "Workbench", title: "Workbench · Forge" },
    management: { crumb: "Manage projects", title: "Manage projects · Forge" },
    portfolio: { crumb: "Portfolio", title: "Portfolio · Forge" },
    delivery: { crumb: "Delivery", title: "Delivery · Forge" },
    unknown: { crumb: "Page not found", title: "Page not found · Forge" },
  };

  function normalizePath(pathname) {
    const trimmed = pathname.replace(/\/+$/, "");
    return trimmed === "" ? "/" : trimmed;
  }

  function viewForPath(pathname) {
    return VIEW_BY_PATH[normalizePath(pathname)] || "projects";
  }

  function isRoutePath(pathname) {
    return Object.prototype.hasOwnProperty.call(VIEW_BY_PATH, normalizePath(pathname));
  }

  function splitRoute(path) {
    const queryIndex = path.indexOf("?");
    if (queryIndex === -1) return { pathname: path, search: "" };
    return { pathname: path.slice(0, queryIndex), search: path.slice(queryIndex) };
  }

  // Last view `renderRoute` announced via focus. Guards focus-on-route-
  // change: only a view *switch* moves focus to `#main-content`, so
  // same-view `?project=` reconciliations (reload, back/forward across
  // two managed ids, fleet row actions) never steal focus out of the
  // operator's current control. Starts null so the boot render announces.
  let lastRouteView = null;

  function renderRoute() {
    // Unknown pathnames render an honest empty state, never the fleet: a
    // pathname outside the route allowlist is not the projects view, so no
    // sidebar link claims it and the crumb/title name the miss. Server-side
    // unknown paths still 404 on hard load; this branch covers any
    // client-side unknown pathname (history entries, hand-built URLs).
    const known = isRoutePath(window.location.pathname);
    const view = known ? viewForPath(window.location.pathname) : "unknown";
    for (const [name, id] of Object.entries(VIEW_IDS)) {
      const element = document.getElementById(id);
      if (element) element.hidden = name !== view;
    }
    const unknown = document.getElementById("view-unknown");
    if (unknown) unknown.hidden = known;
    for (const link of document.querySelectorAll(".sidebar nav .nav-link")) {
      const active = known && link.dataset.route === view;
      link.classList.toggle("nav-active", active);
      if (active) link.setAttribute("aria-current", "page");
      else link.removeAttribute("aria-current");
    }
    const chrome = VIEW_CRUMBS[view] || VIEW_CRUMBS.unknown;
    const crumb = document.getElementById("topbar-crumb");
    if (crumb) crumb.textContent = chrome.crumb;
    document.title = chrome.title;
    // The management view carries an optional `?project=` deep link (see
    // `applyManagementProjectParam`): with the key present the scoped card
    // leads and the bulk table hides; without it the bulk view is exact.
    // Reconciling on every route render keeps reload and back/forward
    // honest; candidate matching waits for workspace discovery.
    if (view === "management") applyManagementProjectParam();
    // The workbench carries the same optional `?project=` deep link (see
    // `applyWorkbenchProjectParam`): a managed id selects and loads that
    // project. The helper is a no-op until the fleet has populated the
    // project selector, and never touches anything for a bare `/workbench`.
    if (view === "workbench") applyWorkbenchProjectParam();
    // Returning to the projects view restores the operator's typed
    // search/filter context (see `restoreFilterState`) and re-applies it.
    // The boot render (`lastRouteView` still null) skips this: the
    // dashboard boot below restores before its first table render, and an
    // early catalog fetch here would race authentication. Same-view
    // `?project=` reconciliations never restore, so deep-link focus
    // behavior is intact.
    if (view === "projects" && lastRouteView !== null && view !== lastRouteView) {
      if (restoreFilterState()) renderProjectsFromInputs();
    }
    // Focus-on-route-change: on a view *switch* only, move focus to the
    // main content region so screen-reader users hear the new view's
    // landmark. `preventScroll` preserves back/forward scroll restoration;
    // the null-guard keeps shells without `#main-content` (login) safe.
    // URL, history, and param handling above are untouched.
    if (view !== lastRouteView) {
      lastRouteView = view;
      const main = document.getElementById("main-content");
      if (main) {
        if (!main.hasAttribute("tabindex")) main.setAttribute("tabindex", "-1");
        try { main.focus({ preventScroll: true }); } catch (_) { /* non-focusable shell: view already shown */ }
      }
    }
  }

  function navigateTo(path) {
    const next = splitRoute(path);
    // Snapshot the operator's typed filter/search context before leaving:
    // the snapshot restores on boot and on switching back into the
    // projects view (see `restoreFilterState`). `?project=` selection
    // is never snapshotted — the URL stays its source of truth.
    persistFilterState();
    // The query string is part of the address: a same-view navigation that
    // only changes `?project=` (fleet "Manage" rows) must still push state so
    // reload and back/forward see the project the operator picked.
    if (
      normalizePath(window.location.pathname) !== normalizePath(next.pathname) ||
      window.location.search !== next.search
    ) {
      window.history.pushState(null, "", next.pathname + next.search);
    }
    renderRoute();
  }

  function initRouter() {
    window.addEventListener("popstate", renderRoute);
    document.addEventListener("click", (event) => {
      if (event.defaultPrevented || event.button !== 0) return;
      if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      const anchor = event.target.closest("a");
      if (!anchor || anchor.target === "_blank" || anchor.hasAttribute("download")) return;
      const url = new URL(anchor.href, window.location.href);
      if (url.origin !== window.location.origin || !isRoutePath(url.pathname)) return;
      event.preventDefault();
      navigateTo(url.pathname + url.search);
    });
  }

  // ---- Filter/search state preservation (slice 4) -----------------------
  //
  // The fleet search box, source filter, and six catalog predicate inputs
  // keep DOM values while the session lives, but a reload wipes them and
  // back/forward never restores what the operator typed. This snapshot
  // (tab-scoped `sessionStorage`, typed filter strings only) restores on
  // dashboard boot and on switching back into the projects view, then
  // re-runs the fleet render path. Project selection is deliberately
  // excluded: `?project=` is always read from the URL, never storage.
  const FILTER_STATE_KEY = "forge.filter-state.v1";
  const FILTER_STATE_IDS = [
    "project-search",
    "source-filter",
    "filter-language",
    "filter-lifecycle",
    "filter-profile",
    "filter-compose",
    "filter-ci",
    "filter-tag",
  ];

  function readFilterState() {
    const state = {};
    for (const id of FILTER_STATE_IDS) {
      const field = document.getElementById(id);
      if (field) state[id] = field.value;
    }
    return state;
  }

  function persistFilterState() {
    try {
      window.sessionStorage.setItem(FILTER_STATE_KEY, JSON.stringify(readFilterState()));
    } catch (_) { /* storage denied: live input values rule */ }
  }

  // Applies the snapshot to the live inputs; true when at least one value
  // changed. Missing inputs and unparseable stores degrade to a no-op.
  function restoreFilterState() {
    let saved = null;
    try {
      saved = JSON.parse(window.sessionStorage.getItem(FILTER_STATE_KEY) || "null");
    } catch (_) {
      return false;
    }
    if (!saved || typeof saved !== "object") return false;
    let changed = false;
    for (const id of FILTER_STATE_IDS) {
      const field = document.getElementById(id);
      if (!field || typeof saved[id] !== "string") continue;
      if (field.value !== saved[id]) {
        field.value = saved[id];
        changed = true;
      }
    }
    return changed;
  }

  // Re-apply the fleet render path after a restore: active predicates
  // re-query the catalog, otherwise the local search/source render runs.
  function renderProjectsFromInputs() {
    if (fleetFiltersActive()) refreshFleetFilters();
    else renderProjects(window.__forgeProjects || []);
  }

  function initFilterStatePersistence() {
    for (const id of FILTER_STATE_IDS) {
      const field = document.getElementById(id);
      if (!field) continue;
      field.addEventListener("input", persistFilterState);
      field.addEventListener("change", persistFilterState);
    }
  }

  async function loginPage() {
    const form = document.getElementById("login-form");
    const error = document.getElementById("login-error");
    const setup = document.getElementById("setup-state");
    // Where to return after sign-in: the `?next=` dashboard route the
    // unauthenticated dashboard boot carried here, or null for the default
    // `index.html`. Only a same-origin path on the dashboard route
    // allowlist is honored — anything else (absolute URL, foreign origin,
    // unknown path, the login page itself) is dropped so `next` can never
    // navigate off-origin.
    const loginNextTarget = () => {
      const raw = (new URLSearchParams(window.location.search).get("next") || "").trim();
      if (raw === "") return null;
      let url;
      try {
        url = new URL(raw, window.location.origin);
      } catch (_) {
        return null;
      }
      if (url.origin !== window.location.origin || !isRoutePath(url.pathname)) return null;
      return url.pathname + url.search;
    };
    const next = loginNextTarget();
    try {
      const state = await session();
      if (state.authenticated) { window.location.replace(next || "index.html"); return; }
      if (!state.configured) {
        setup.hidden = false;
        setup.textContent = `Set up your Forge administrator from a terminal with: ${state.setup_command || "forge identity setup --email you@example.com"}`;
        form.hidden = true;
      }
    } catch (_) {
      setup.hidden = false;
      setup.classList.add("notice-error");
      setup.textContent = "Forge API is unavailable. Start the API, then reload this page.";
    }
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      error.hidden = true;
      clearErrorSummary("login-error-summary");
      clearFieldError("login-email", "login-error");
      clearFieldError("login-password", "login-error");
      const button = form.querySelector("button[type=submit]");
      button.disabled = true;
      button.textContent = "Signing in…";
      try {
        await request("/v1/admin/session", {
          method: "POST",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ email: form.elements.email.value, password: form.elements.password.value }),
        });
        form.elements.password.value = "";
        window.location.assign(next || "index.html");
      } catch (_) {
        const message = "Email or password is incorrect.";
        error.textContent = message;
        error.hidden = false;
        setFieldError("login-email", "login-error", message);
        setFieldError("login-password", "login-error", message);
        renderErrorSummary("login-error-summary", "There is a problem signing in", [
          { fieldId: "login-email", label: "Email address", message },
          { fieldId: "login-password", label: "Password", message },
        ]);
        form.elements.password.value = "";
      } finally {
        button.disabled = false;
        button.innerHTML = 'Sign in to your workspace <span aria-hidden="true">→</span>';
      }
    });
    // Password show/hide: flips only the input type and the toggle state.
    // The input's name, id, and autocomplete never change, and nothing
    // blocks paste, so password managers keep working.
    const passwordToggle = document.getElementById("login-password-toggle");
    if (passwordToggle) {
      passwordToggle.addEventListener("click", () => {
        const input = document.getElementById("login-password");
        if (!input) return;
        const show = input.type === "password";
        input.type = show ? "text" : "password";
        passwordToggle.textContent = show ? "Hide" : "Show";
        passwordToggle.setAttribute("aria-pressed", String(show));
      });
    }
  }

  function textCell(value, className) {
    const cell = document.createElement("td");
    if (className) cell.className = className;
    cell.textContent = value || "—";
    return cell;
  }

  function makeBadge(text, kind) {
    const badge = document.createElement("span");
    badge.className = `badge badge-${kind}`;
    badge.textContent = text;
    return badge;
  }

  const SOURCE_LABELS = { self: "Forge (self)", registry: "Registered", inventory: "Inventory", fleet: "Workspace", published: "Published (Mac)" };
  const SOURCE_BADGE = { self: "self", registry: "registry", inventory: "inventory", fleet: "workspace", published: "published" };
  const FLEET_STATUS_LABELS = { available: "Available", stale: "Stale", unconfigured: "Not configured", unavailable: "Unavailable" };
  const CATEGORY_LABELS = {
    registry: "Registry", creation: "Creation", quality: "Quality", release: "Release",
    delivery: "Delivery", portfolio: "Portfolio", identity: "Identity", transports: "Transports", reference: "Reference",
  };
  const AVAILABILITY_LABELS = {
    web: "Web", cli_only: "CLI only", provider_required: "Provider required",
    project_capability_required: "Project capability required", disabled: "Disabled", not_yet_web: "Not in web yet",
  };
  const AVAILABILITY_BADGE = {
    web: "cat-web", cli_only: "cat-cli", provider_required: "cat-provider",
    project_capability_required: "cat-capability", disabled: "cat-disabled", not_yet_web: "cat-gap",
  };
  const RISK_LABELS = { read: "Read", local_write: "Local write", remote_write: "Remote write", session_admin: "Session admin" };
  const RISK_BADGE = { read: "risk-read", local_write: "risk-local", remote_write: "risk-remote", session_admin: "risk-session" };

  function renderSources(sources) {
    const list = document.getElementById("source-list");
    list.replaceChildren();
    for (const source of sources) {
      const item = document.createElement("li");
      item.className = `source-item source-${source.status}`;
      const head = document.createElement("div"); head.className = "source-head";
      const name = document.createElement("strong"); name.textContent = SOURCE_LABELS[source.id] || source.id;
      head.append(name, makeBadge(FLEET_STATUS_LABELS[source.status] || source.status, source.status));
      const meta = document.createElement("span"); meta.className = "source-meta";
      const count = Number(source.count || 0);
      meta.textContent = count === 0 ? "No entries" : `${count} entr${count === 1 ? "y" : "ies"}`;
      head.append(meta);
      item.append(head);
      if (source.reason) { const reason = document.createElement("p"); reason.className = "source-reason"; reason.textContent = source.reason; item.append(reason); }
      if (source.malformed?.length) { const bad = document.createElement("p"); bad.className = "source-malformed"; bad.textContent = `${source.malformed.length} entr${source.malformed.length === 1 ? "y" : "ies"} skipped as malformed`; item.append(bad); }
      if (source.observed_at) { const seen = document.createElement("p"); seen.className = "source-seen"; seen.textContent = `Observed ${source.observed_at}`; item.append(seen); }
      list.append(item);
    }
  }

  const PROFILE_WORDS = {
    "rust-web": "Rust web", "nextjs-web": "Next.js web",
    "react-web": "React web", "aspnet-web": "ASP.NET web",
    "flutter-app": "Flutter app", "python-service": "Python service",
  };

  function profileWords(profile) {
    return PROFILE_WORDS[profile] || profile || "Unknown type";
  }

  // One plain status line per project: health first, then anything that
  // needs attention. No codes, no hashes, no source labels.
  function projectStatusLine(project) {
    if (project.conflict) return "Needs attention: another entry uses this name";
    const publish = project.publish || {};
    if (publish.health === false || publish.status === "failed") {
      return "Publishing needs attention";
    }
    if (project.freshness === "stale") return "Showing saved data (source is stale)";
    if (project.state === "done") return "Up to date";
    if (project.state && project.state !== "—") return project.state;
    return "No recent activity";
  }

  function renderProjects(projects) {
    const body = document.getElementById("project-rows");
    const query = document.getElementById("project-search").value.trim().toLowerCase();
    const sourceFilter = document.getElementById("source-filter").value;
    const filtered = projects.filter((project) => {
      if (sourceFilter && project.source !== sourceFilter) return false;
      // The catalog predicate set, fetched from GET /v1/projects/catalog:
      // the same Core query `forge project list` runs for the same
      // predicate. Null means no predicate is active.
      if (fleetFilterIds && !fleetFilterIds.has(project.identity)) return false;
      return `${project.name} ${project.identity} ${project.profile}`.toLowerCase().includes(query);
    });
    body.replaceChildren();
    for (const project of filtered) {
      const row = document.createElement("tr");
      if (project.is_self) row.className = "row-self";
      if (project.conflict) row.className = "row-conflict";

      const identity = document.createElement("td");
      const name = document.createElement("div"); name.className = "project-name";
      const avatar = document.createElement("span"); avatar.className = "project-avatar"; avatar.setAttribute("aria-hidden", "true"); avatar.textContent = ((project.name || project.identity) || "?").slice(0, 1).toUpperCase();
      const label = document.createElement("span"); label.textContent = project.name || `(${project.identity})`;
      name.append(avatar, label);
      if (project.is_self) name.append(makeBadge("This is Forge", "self"));
      if (project.conflict) name.append(makeBadge("Needs attention", "conflict"));
      identity.append(name);
      if (project.name && project.name !== project.identity) {
        const ref = document.createElement("div"); ref.className = "muted"; ref.textContent = project.identity;
        identity.append(ref);
      }
      row.append(identity);

      const details = (project.is_self ? "Forge itself" : profileWords(project.profile))
        + (project.management === "managed" || project.is_self ? "" : " · not managed yet");
      row.append(textCell(details));

      const state = document.createElement("td");
      const stateLabel = document.createElement("span"); stateLabel.className = "state-label";
      const dot = document.createElement("span"); dot.className = `state-dot ${project.conflict || (project.publish && (project.publish.health === false || project.publish.status === "failed")) ? "state-failed" : (project.state === "done" ? "state-observed" : "")} ${project.freshness === "stale" ? "state-stale" : ""}`; dot.setAttribute("aria-hidden", "true");
      stateLabel.append(dot, document.createTextNode(projectStatusLine(project))); state.append(stateLabel); row.append(state);

      const action = document.createElement("td");
      if (project.management === "managed" || project.is_self) {
        const open = el("button", "button button-quiet", "Open");
        open.type = "button";
        open.setAttribute("aria-label", `Open ${project.name} in the workbench`);
        open.addEventListener("click", () => openInWorkbench(project.identity));
        action.append(open);
      } else if (!project.conflict) {
        const link = document.createElement("a");
        // Carry the project identity: the management view reads `?project=`
        // on load and ticks that project's workspace row. A bare
        // `/management` link cannot say which project the operator picked.
        link.href = `/management?project=${encodeURIComponent(project.identity)}`;
        link.textContent = "Manage";
        link.setAttribute("aria-label", `Manage ${project.name}: onboard it from the workspace panel`);
        action.append(link);
      } else {
        action.append(textCell("—"));
      }
      row.append(action);

      body.append(row);
    }
    document.getElementById("no-results").hidden = projects.length === 0 || filtered.length > 0;
    document.querySelector(".table-scroll").hidden = filtered.length === 0;
    document.getElementById("project-count").textContent = `${filtered.length} of ${projects.length} project${projects.length === 1 ? "" : "s"}`;
  }

  function openInWorkbench(identity) {
    // Carry the project identity: the workbench view reads `?project=` on
    // load and boots that managed project. A bare `/workbench` navigation
    // cannot say which project the operator picked, is lost on reload, and
    // is invisible to back/forward. The route render applies the load.
    navigateTo(`/workbench?project=${encodeURIComponent(identity)}`);
  }

  // ---- Fleet filter row (catalog predicates) -----------------------------
  //
  // The six inputs above the project table carry the catalog's existing
  // predicates — language, lifecycle, profile, compose, CI, tag — wired to
  // `GET /v1/projects/catalog`, which already accepts all of them. The
  // response's project ids constrain the fleet table; the free-text search
  // and the source filter keep applying on top, so all three compose. No
  // new endpoint: the browser only reads the shared Core query.
  let fleetFilterIds = null;
  let fleetFilterTimer = null;

  const FLEET_FILTER_FIELDS = [
    ["filter-language", "language"],
    ["filter-lifecycle", "lifecycle"],
    ["filter-profile", "profile"],
    ["filter-compose", "compose"],
    ["filter-ci", "ci"],
    ["filter-tag", "tag"],
  ];

  function fleetFilterParams() {
    const params = new URLSearchParams();
    for (const [id, key] of FLEET_FILTER_FIELDS) {
      const value = document.getElementById(id).value.trim();
      if (value) params.append(key, value);
    }
    params.append("limit", "1000");
    return params;
  }

  function fleetFiltersActive() {
    return FLEET_FILTER_FIELDS.some(([id]) => document.getElementById(id).value.trim() !== "");
  }

  function showFleetFilterError(message) {
    const error = document.getElementById("fleet-filter-error");
    if (!message) { error.hidden = true; error.textContent = ""; return; }
    error.textContent = message;
    error.hidden = false;
  }

  async function refreshFleetFilters() {
    showFleetFilterError("");
    if (!fleetFiltersActive()) {
      fleetFilterIds = null;
      renderProjects(window.__forgeProjects || []);
      return;
    }
    let data;
    try {
      data = await request(`/v1/projects/catalog?${fleetFilterParams().toString()}`, {
        headers: { Accept: "application/json" },
      });
    } catch (err) {
      // A failed predicate read must never present an unfiltered table as
      // a filtered one: the constraint is lifted and the failure is shown.
      fleetFilterIds = null;
      showFleetFilterError(`Catalog filters are unavailable right now — showing all projects. (${(err && err.message) || "request failed"})`);
      renderProjects(window.__forgeProjects || []);
      return;
    }
    const records = (data.catalog && data.catalog.records) || [];
    fleetFilterIds = new Set(records.map((record) => record.project_id));
    renderProjects(window.__forgeProjects || []);
  }

  function initFleetFilters() {
    for (const [id] of FLEET_FILTER_FIELDS) {
      document.getElementById(id).addEventListener("input", () => {
        window.clearTimeout(fleetFilterTimer);
        fleetFilterTimer = window.setTimeout(refreshFleetFilters, 250);
      });
    }
    document.getElementById("filter-clear").addEventListener("click", () => {
      for (const [id] of FLEET_FILTER_FIELDS) document.getElementById(id).value = "";
      window.clearTimeout(fleetFilterTimer);
      refreshFleetFilters();
    });
  }

  function loadCommands() {
    // The catalog is static descriptive metadata. A failure here never
    // hides the project fleet, and the workbench renders an honest
    // empty state for the actions card. The catalog is consumed by
    // the workbench to derive per-project buttons; the standalone
    // catalog-as-reference page is gone.
    return request("/v1/admin/commands", { headers: { Accept: "application/json" } })
      .then((data) => {
        catalogCommands = data.commands || [];
        if (workbench.id) renderProjectActions();
        renderManagementActions();
      })
      .catch(() => {
        catalogCommands = [];
        if (workbench.id) renderProjectActions();
        renderManagementActions();
      });
  }

  // ---- Project workbench (single managed project) ------------------------
  //
  // Boundary: the browser only ever sends a validated project `id` chosen
  // from the dropdown it built from the fleet view. It never sends a
  // filesystem path, a command line, or anything a shell would interpret.
  // Every action here calls a typed in-process JSON endpoint; refused or
  // unavailable workflows are rendered as honest state, never as a fake
  // success and never as an executable control.
  const HEALTH_LABELS = { healthy: "Healthy", stale: "Stale", issues: "Has issues", unavailable: "Unavailable" };
  const HEALTH_BADGE = { healthy: "state-done", stale: "state-stale", issues: "state-failed", unavailable: "state-observed" };
  const workbench = { id: null, digest: null, idempotencyKey: null, delivery: null };
  // The last `?project=` value the workbench applied itself. Guards the
  // route render against re-fetching the detail on unrelated re-renders
  // while still following a changed parameter (reload, back/forward, fleet
  // "Open" on another row).
  let wbAutoParam = null;
  // True once the fleet has populated the workbench selector (even with
  // zero managed projects). The `?project=` apply waits for this instead
  // of mistaking "not loaded yet" for "no such project".
  let wbFleetReady = false;
  // The catalog rows fetched once by `loadCommands`; the workbench renders a
  // runnable confirm-gated control for every row that carries an `execution`
  // block, so the inventory itself — not a hard-wired widget — is actionable.
  let catalogCommands = [];

  function managedProjects(projects) {
    return projects.filter((project) => project.management === "managed" || project.management === "self");
  }

  function detailRow(term, value) {
    const row = document.createElement("div");
    row.className = "detail-row";
    const dt = document.createElement("dt"); dt.textContent = term;
    const dd = document.createElement("dd"); dd.textContent = value === null || value === undefined || value === "" ? "—" : String(value);
    row.append(dt, dd);
    return row;
  }

  function renderManifest(manifest) {
    const list = document.getElementById("wb-manifest");
    list.replaceChildren(
      detailRow("Project id", manifest.id),
      detailRow("Name", manifest.name),
      detailRow("Profile", manifest.profile),
      detailRow("Stack", manifest.stack),
      detailRow("Maturity", manifest.maturity),
      detailRow("Target maturity", manifest.target_maturity),
      detailRow("Platform version", manifest.platform_version),
      detailRow("Kit", manifest.kit_id ? `${manifest.kit_id} (${manifest.kit_version || "unversioned"})` : null),
      detailRow("Deployment target", manifest.deployment_target),
      detailRow("Runtime", manifest.runtime),
      detailRow("Quality", manifest.quality_status),
      detailRow("Docs", manifest.docs_status),
    );
    const features = document.getElementById("wb-features");
    features.replaceChildren();
    const entries = Object.entries(manifest.features || {});
    if (!entries.length) {
      const chip = document.createElement("span"); chip.className = "evidence-chip"; chip.textContent = "No installed features"; features.append(chip);
    }
    for (const [name, version] of entries) {
      const chip = document.createElement("span"); chip.className = "evidence-chip"; chip.textContent = `${name}: ${version}`; features.append(chip);
    }
  }

  function renderHealth(health) {
    const box = document.getElementById("wb-health");
    box.replaceChildren();
    const state = health.state || "unavailable";
    const label = document.createElement("div"); label.className = "wb-health-state";
    const dot = document.createElement("span"); dot.className = `state-dot ${HEALTH_BADGE[state] || ""}`; dot.setAttribute("aria-hidden", "true");
    const text = document.createElement("strong"); text.textContent = HEALTH_LABELS[state] || state;
    label.append(dot, text);
    box.append(label);
    if (health.note) { const note = document.createElement("p"); note.className = "muted"; note.textContent = health.note; box.append(note); }
    const findings = health.findings || [];
    const blocking = findings.filter((f) => f.status === "fail" || f.status === "unavailable" || f.status === "FAIL" || f.status === "UNAVAILABLE");
    if (blocking.length) {
      const ul = document.createElement("ul"); ul.className = "wb-findings";
      for (const finding of blocking) {
        const li = document.createElement("li"); li.textContent = `${finding.id || "finding"} — ${finding.summary || finding.reason || "blocking"}`;
        ul.append(li);
      }
      box.append(ul);
    } else if (state === "healthy" || state === "stale") {
      const ok = document.createElement("p"); ok.className = "muted"; ok.textContent = "No blocking findings."; box.append(ok);
    }
  }

  // The read-only project status projection: one overall state plus its
  // doctor/check/readiness sub-checks. Rendered as text only; the server
  // never returns an absolute path or raw finding detail.
  const STATUS_CHECK_LABELS = { doctor: "Doctor", check: "Checker", readiness: "Readiness" };

  function renderProjectStatus(status) {
    const box = document.getElementById("wb-status");
    if (!box) return;
    box.replaceChildren();
    const state = status.state || "unavailable";
    const head = document.createElement("div"); head.className = "wb-health-state";
    const dot = document.createElement("span"); dot.className = `state-dot ${HEALTH_BADGE[state] || ""}`; dot.setAttribute("aria-hidden", "true");
    const text = document.createElement("strong"); text.textContent = HEALTH_LABELS[state] || state;
    head.append(dot, text);
    box.append(head);
    const checks = status.checks || [];
    if (checks.length) {
      const ul = document.createElement("ul"); ul.className = "wb-findings";
      for (const check of checks) {
        const li = document.createElement("li");
        const name = STATUS_CHECK_LABELS[check.id] || check.id || "check";
        const label = `${name} — ${HEALTH_LABELS[check.state] || check.state || "unavailable"}`;
        li.textContent = check.reason ? `${label}: ${check.reason}` : `${label}: ${check.summary || "no blocking evidence"}`;
        ul.append(li);
      }
      box.append(ul);
    }
    if (status.note) { const note = document.createElement("p"); note.className = "muted"; note.textContent = status.note; box.append(note); }
  }

  async function loadProjectStatus(id) {
    const box = document.getElementById("wb-status");
    if (box) box.replaceChildren();
    let status;
    try {
      status = await request(`/v1/admin/projects/${encodeURIComponent(id)}/status`, { headers: { Accept: "application/json" } });
    } catch (err) {
      if (box) {
        const note = document.createElement("p"); note.className = "muted";
        note.textContent = (err && err.message) ? err.message : "Project status is unavailable right now.";
        box.replaceChildren(note);
      }
      return;
    }
    renderProjectStatus(status);
  }

  const DELIVERY_ACTION_LABELS = {
    "delivery-preflight": "Delivery preflight",
    "delivery-stage": "Delivery stage",
    "delivery-promote": "Delivery promote",
    "delivery-hermora-retry": "Hermora retry",
  };

  function deliveryVerbSummary(name, verb) {
    if (!verb || verb.op_id === undefined || verb.op_id === null) return `${name}: not yet recorded`;
    return `${name}: ${verb.state === "done" ? "done" : (verb.state || "needs attention")}`;
  }

  function renderProjectDelivery(report, next) {
    const box = document.getElementById("wb-delivery");
    if (!box) return;
    box.replaceChildren();
    if (!report) {
      box.append(el("p", "muted", "Delivery status is unavailable right now."));
      return;
    }
    const summary = el("p", "wb-plan-head", `Phase: ${report.phase || "unknown"}`);
    box.append(summary);
    const facts = el("dl", "detail-list");
    const addFact = (term, value) => {
      const name = el("dt", null, term);
      const detail = el("dd", null, value || "—");
      facts.append(name, detail);
    };
    addFact("Revision", report.revision ? report.revision.slice(0, 12) : null);
    addFact("Environment", report.environment || null);
    addFact("Updated", report.updated_at || null);
    box.append(facts);
    const verbs = el("ul", "wb-findings");
    for (const [name, verb] of [["Preflight", report.preflight], ["Stage", report.stage], ["Promote", report.promote]]) {
      const item = el("li", null, deliveryVerbSummary(name, verb));
      verbs.append(item);
    }
    const hermora = report.hermora || {};
    const hermoraItem = el("li", null, hermora.op_id === undefined || hermora.op_id === null
      ? "Hermora: not yet recorded"
      : `Hermora: ${hermora.state === "done" ? "done" : (hermora.state || "needs attention")}`);
    verbs.append(hermoraItem);
    box.append(verbs);
    if (next && next.action) {
      const action = el("p", "muted", `Next: ${DELIVERY_ACTION_LABELS[next.action] || next.action}. ${(next.guidance || "Use the matching control below.")}`);
      box.append(action);
      if (next.blocked) {
        const blocked = el("p", "muted", "This step is blocked until the failed staged evidence is resolved.");
        box.append(blocked);
      }
    }
  }

  // The latest delivery status for the open project, kept so action
  // controls can pre-fill their confirmations — the operator never
  // hand-copies operation numbers or revisions.
  async function loadProjectDelivery(id) {
    const box = document.getElementById("wb-delivery");
    if (box) box.replaceChildren(el("p", "muted", "Loading delivery status…"));
    workbench.delivery = null;
    try {
      const data = await request(`/v1/admin/projects/${encodeURIComponent(id)}/delivery/status`, { headers: { Accept: "application/json" } });
      workbench.delivery = data.next || null;
      renderProjectDelivery(data.delivery_status, data.next);
      // Re-render action controls so staged confirmations arrive pre-filled.
      // This runs only on project open and after a completed action, when no
      // preview is in progress.
      renderProjectActions();
    } catch (err) {
      if (box) box.replaceChildren(el("p", "muted", (err && err.message) ? err.message : "Delivery status is unavailable right now."));
    }
  }

  // ---- Maintain card (per-project maintainer surface) --------------------
  //
  // Above the read-only cards: the GitHub observation as Forge last saw it
  // (with its freshness, or an honest unavailable-with-reason — never empty
  // fields), the derived classification proposals with per-field
  // approve/reject, and one action applying the approved set through the
  // plugins. The approve/reject/apply controls are the catalog-driven
  // `buildActionControl` cards, so they inherit the preview → confirm →
  // apply discipline and the typed-field guarantees; the per-proposal
  // buttons only prefill and open the matching card.
  function maintainPath(id) {
    return `/v1/admin/projects/${encodeURIComponent(id)}/maintain`;
  }

  async function loadMaintain(id) {
    const body = document.getElementById("wb-maintain-body");
    const actions = document.getElementById("wb-maintain-actions");
    if (!body || !actions) return;
    body.replaceChildren(el("p", "muted", "Loading maintainer data…"));
    actions.replaceChildren();
    let data;
    try {
      data = await request(maintainPath(id), { headers: { Accept: "application/json" } });
    } catch (err) {
      body.replaceChildren(el("p", "muted", (err && err.message) || "Maintainer data is unavailable right now."));
      return;
    }
    renderMaintainBody(data);
    renderMaintainActions();
  }

  async function reloadMaintainBody() {
    if (!workbench.id) return;
    try {
      const data = await request(maintainPath(workbench.id), { headers: { Accept: "application/json" } });
      renderMaintainBody(data);
    } catch (_) {
      // The action card already reports its own outcome; stale proposals
      // stay visible until the next refresh rather than being wiped.
    }
  }

  function renderMaintainBody(data) {
    const body = document.getElementById("wb-maintain-body");
    body.replaceChildren();
    body.append(renderMaintainObservation(data.github || {}));
    body.append(renderMaintainProposals(data.proposals || []));
    body.append(renderMaintainPlugins(data.plugins || []));
  }

  function renderMaintainObservation(github) {
    const wrap = el("div", "wb-maintain-section");
    wrap.append(el("h4", "wb-maintain-head", "GitHub observation"));
    // A blank field and an unreachable field are different facts: any
    // state other than current/stale renders the reason, never empty rows.
    if (github.state !== "current" && github.state !== "stale") {
      const box = el("p", "muted");
      box.textContent = github.reason
        ? `GitHub remote unavailable — ${github.reason}`
        : "GitHub remote unavailable.";
      wrap.append(box);
      return wrap;
    }
    const list = el("dl", "detail-list");
    const add = (term, value) => {
      const name = el("dt", null, term);
      const detail = el("dd", null, (value === null || value === undefined || value === "" || (Array.isArray(value) && !value.length)) ? "—" : String(Array.isArray(value) ? value.join(", ") : value));
      list.append(name, detail);
    };
    add("Description", github.description);
    add("Topics", github.topics);
    add("Homepage", github.homepage);
    add("Language", github.language);
    add("Freshness", `${github.freshness || "unknown"}${github.observed_at ? ` · observed ${github.observed_at}` : ""}`);
    wrap.append(list);
    return wrap;
  }

  function renderMaintainProposals(proposals) {
    const wrap = el("div", "wb-maintain-section");
    wrap.append(el("h4", "wb-maintain-head", "Derived proposals"));
    if (!proposals.length) {
      wrap.append(el("p", "muted", "No derived proposals yet. Run `forge classify derive` in a terminal to derive some from this project's own evidence."));
      return wrap;
    }
    const tableWrap = el("div", "table-scroll compact");
    tableWrap.setAttribute("tabindex", "0");
    tableWrap.setAttribute("role", "region");
    tableWrap.setAttribute("aria-label", "Derived classification proposals");
    const table = el("table");
    const head = el("thead");
    const headRow = el("tr");
    for (const label of ["Kind", "State", "Confidence", "Evidence", "Suggested", "Decision"]) {
      const th = el("th", null, label);
      th.setAttribute("scope", "col");
      headRow.append(th);
    }
    head.append(headRow);
    table.append(head);
    const body = el("tbody");
    for (const proposal of proposals) {
      const row = el("tr");
      row.append(textCell(proposal.kind), textCell(proposal.state), textCell(proposal.confidence));
      const evidence = proposal.evidence_count === undefined || proposal.evidence_count === null
        ? "—"
        : String(proposal.evidence_count);
      row.append(textCell(evidence), textCell(proposal.suggested_at || "—"));
      const decision = el("td");
      if (proposal.state === "Suggested" || proposal.state === "suggested") {
        const approve = el("button", "button button-quiet wb-maintain-decide", "Approve");
        approve.type = "button";
        approve.dataset.proposal = proposal.id;
        approve.dataset.decision = "classify.approve";
        approve.setAttribute("aria-label", `Approve proposal ${proposal.id}`);
        const reject = el("button", "button button-quiet wb-maintain-decide", "Reject");
        reject.type = "button";
        reject.dataset.proposal = proposal.id;
        reject.dataset.decision = "classify.reject";
        reject.setAttribute("aria-label", `Reject proposal ${proposal.id}`);
        decision.append(approve, reject);
      } else {
        decision.textContent = "—";
      }
      row.append(decision);
      body.append(row);
    }
    table.append(body);
    tableWrap.append(table);
    wrap.append(tableWrap);
    for (const button of wrap.querySelectorAll(".wb-maintain-decide")) {
      button.addEventListener("click", () => openMaintainDecision(button.dataset.decision, button.dataset.proposal));
    }
    return wrap;
  }

  function renderMaintainPlugins(plugins) {
    const wrap = el("div", "wb-maintain-section");
    wrap.append(el("h4", "wb-maintain-head", "Plugins"));
    if (!plugins.length) {
      wrap.append(el("p", "muted", "No plugins configured — the apply action will refuse until a metadata plugin is declared."));
      return wrap;
    }
    const list = el("ul", "wb-plan-steps");
    for (const plugin of plugins) {
      const state = plugin.state && plugin.state !== "available" && plugin.reason
        ? ` — ${plugin.state}: ${plugin.reason}`
        : ` — ${plugin.state || (plugin.enabled ? "enabled" : "disabled")}`;
      list.append(el("li", null, `${plugin.id} (${plugin.kind || "unknown kind"})${state}`));
    }
    wrap.append(list);
    return wrap;
  }

  // Prefill the matching Maintain action card with this proposal and open
  // it, so the operator reviews the typed preview and confirms — the
  // buttons never decide anything by themselves.
  function openMaintainDecision(commandId, proposalId) {
    const card = document.querySelector(`#wb-maintain-actions .wb-action-card[data-command="${commandId}"]`);
    if (!card) {
      showWorkbenchNotice("The Maintain actions are unavailable right now — the command catalog could not be loaded.");
      return;
    }
    const body = card.querySelector(".wb-action-body");
    if (body && body.hidden) card.querySelector(".wb-action-head").click();
    const field = card.querySelector('.wb-action-body input[type="text"]');
    if (field) {
      field.value = proposalId;
      field.focus();
    }
    scrollIntoViewRespectingMotion(card, { block: "center" });
  }

  // The three Maintain actions, rendered from the command catalog with no
  // bespoke wiring: per-field approve, per-field reject, and the one apply
  // of the approved set. A catalog that has not loaded leaves an honest
  // empty state instead of dead buttons.
  function renderMaintainActions() {
    const box = document.getElementById("wb-maintain-actions");
    box.replaceChildren();
    if (!workbench.id) return;
    const wanted = ["classify.approve", "classify.reject", "classify.apply"];
    const rows = wanted.map((id) => catalogCommands.find(
      (command) => command.id === id && command.execution && command.execution.route.includes("{id}"),
    ));
    if (rows.some((row) => !row)) {
      box.append(el("p", "muted", "Maintain actions are unavailable right now — the command catalog could not be loaded."));
      return;
    }
    for (const command of rows) {
      box.append(buildActionControl(command, {
        projectId: workbench.id,
        onSuccess: () => reloadMaintainBody(),
      }));
    }
  }

  function renderLifecycle(manifest) {    // `GET /v1/admin/projects/{id}` returns `manifest` as a flat, path-free
    // view of the project's own `forge.yaml` — the same object
    // `renderManifest` reads. It carries no filesystem path by design, so the
    // lifecycle card shows the git remote and last commit instead: those are
    // the facts an operator actually uses to identify a project.
    const m = (manifest && manifest.maturity) || "—";
    const t = (manifest && manifest.target_maturity) || "—";
    document.getElementById("wb-lifecycle-maturity").textContent = m;
    document.getElementById("wb-lifecycle-target").textContent = t;
    document.getElementById("wb-lifecycle-profile").textContent =
      (manifest && manifest.profile) || "—";
    document.getElementById("wb-lifecycle-stack").textContent =
      (manifest && manifest.stack) || "—";
    document.getElementById("wb-lifecycle-remote").textContent =
      (manifest && manifest.git_remote) || "—";
    document.getElementById("wb-lifecycle-commit").textContent =
      (manifest && manifest.last_commit) || "—";
    document.getElementById("wb-lifecycle-updated").textContent =
      (manifest && manifest.observed_at) || "—";

    const summary = document.getElementById("wb-lifecycle-summary");
    if (m === "—") {
      summary.textContent =
        "This project has no declared maturity yet. The buttons below are the actions available now; click one to plan, preview, and confirm.";
      return;
    }
    summary.textContent =
      `This project is at maturity ${m} — ${describeMaturity(m)}. ` + nextStep(m, t);
  }

  // What the operator does next. Three distinct cases; collapsing them into one
  // sentence produced "No target maturity is declared" for a project that had
  // declared L1, which is the opposite of the truth.
  function nextStep(current, target) {
    if (target === "—") {
      return "It has no target maturity, so pick one before planning an upgrade.";
    }
    if (target === current) {
      return "Its target maturity is the level it is already at, so there is no maturity gap to close.";
    }
    return `It is working toward ${target}.`;
  }

  // A maturity level names the project lifecycle stage an operator sees in
  // the workbench header. The level itself comes from the manifest schema;
  // the description here is the operator-visible gloss, and it must say what
  // the operator does next rather than restate the level.
  const MATURITY_LABELS = {
    L0: "prototype — nothing is committed yet; adopt it into the registry to start tracking it",
    L1: "scaffolded — the project builds; declare a target maturity and add features",
    L2: "working — features in flight; upgrade dependencies, plan a release, publish when ready",
    L3: "releasing — publish, deploy, and watch delivery until the evidence is green",
    L4: "sustained — keep it healthy; retire it when the work is done",
  };
  function describeMaturity(level) {
    return MATURITY_LABELS[level] || "unknown maturity level";
  }

  function renderOperations(operations) {
    const body = document.getElementById("wb-operations");
    body.replaceChildren();
    if (!operations.length) {
      const row = document.createElement("tr");
      const cell = document.createElement("td"); cell.colSpan = 5; cell.className = "muted"; cell.textContent = "No journal entries for this project yet.";
      row.append(cell); body.append(row);
      return;
    }
    for (const operation of operations) {
      const row = document.createElement("tr");
      row.append(textCell(String(operation.operation_id)), textCell(operation.kind), textCell(operation.state), textCell(operation.started_at), textCell(operation.detail));
      body.append(row);
    }
  }

  function populateFeatures(manifest) {
    const select = document.getElementById("wb-feature");
    select.replaceChildren();
    const all = document.createElement("option"); all.value = ""; all.textContent = "All outdated features"; select.append(all);
    for (const name of Object.keys(manifest.features || {})) {
      const option = document.createElement("option"); option.value = name; option.textContent = name; select.append(option);
    }
  }

  function showWorkbenchNotice(message) {
    const notice = document.getElementById("workbench-notice");
    notice.textContent = message;
    notice.hidden = false;
  }

  function clearWorkbenchNotice() {
    document.getElementById("workbench-notice").hidden = true;
  }

  function resetPlanState() {
    workbench.digest = null;
    workbench.idempotencyKey = null;
    document.getElementById("wb-plan-result").hidden = true;
    document.getElementById("wb-apply-result").hidden = true;
    document.getElementById("wb-apply").disabled = true;
    document.getElementById("wb-confirm-wrap").hidden = true;
    document.getElementById("wb-confirm").checked = false;
  }

  async function loadWorkbenchDetail(id) {
    clearWorkbenchNotice();
    resetPlanState();
    resetProjectActions();
    let data;
    try {
      data = await request(`/v1/admin/projects/${encodeURIComponent(id)}`, { headers: { Accept: "application/json" } });
    } catch (err) {
      document.getElementById("workbench-body").hidden = true;
      document.getElementById("workbench-empty").hidden = false;
      showWorkbenchNotice(err.message || "This project could not be opened in the workbench.");
      return;
    }
    workbench.id = id;
    document.getElementById("workbench-empty").hidden = true;
    document.getElementById("workbench-body").hidden = false;
    renderManifest(data.manifest || {});
    renderHealth(data.health || { state: "unavailable" });
    renderLifecycle(data.manifest || {});
    loadProjectStatus(id);
    loadProjectDelivery(id);
    renderOperations(data.operations || []);
    populateFeatures(data.manifest || {});
    renderProjectActions();
    loadMaintain(id);
    scrollIntoViewRespectingMotion(document.getElementById("workbench-title"), { block: "start" });
  }

  function renderPlan(result) {
    const box = document.getElementById("wb-plan-result");
    box.replaceChildren();
    box.hidden = false;
    const head = document.createElement("p"); head.className = "wb-plan-head";
    head.textContent = `Effect: ${result.effect || "none"} — this plan changes nothing until you confirm it.`;
    box.append(head);
    const steps = (result.plan && result.plan.steps) || [];
    if (steps.length) {
      const ul = document.createElement("ul"); ul.className = "wb-plan-steps";
      for (const step of steps) {
        const li = document.createElement("li");
        const version = step.old_version
          ? `${step.old_version} → ${step.new_version}`
          : `${step.new_version}`;
        li.textContent = `${step.action || "step"} ${step.feature || ""} ${version}`.trim();
        ul.append(li);
      }
      box.append(ul);
    } else {
      const none = document.createElement("p"); none.className = "muted"; none.textContent = "Nothing to upgrade — the project is already at the pinned versions."; box.append(none);
    }
    const digestNote = document.createElement("p"); digestNote.className = "muted";
    digestNote.textContent = "The exact plan is held for confirmation — nothing is written until you apply it.";
    box.append(digestNote);
    workbench.digest = result.plan_digest;
    document.getElementById("wb-apply").disabled = false;
    document.getElementById("wb-confirm-wrap").hidden = false;
  }

  function renderApplyError(result, status) {
    const box = document.getElementById("wb-apply-result");
    box.replaceChildren();
    box.hidden = false;
    const head = document.createElement("p"); head.className = "wb-plan-head";
    head.textContent = result.error ? `${result.error.message}` : "The apply was refused. Nothing was written.";
    box.append(head);
    // A stale digest (409) returns a refreshed plan; show it so the operator
    // can re-review and re-confirm rather than acting blind.
    if (result.plan && result.plan_digest) {
      workbench.digest = result.plan_digest;
      const note = document.createElement("p"); note.className = "muted";
      note.textContent = "A refreshed plan is shown below. Review it and confirm again to apply.";
      box.append(note);
      renderPlan(result);
    } else if (status === 409) {
      document.getElementById("wb-apply").disabled = true;
    }
  }

  function renderApplySuccess(result) {
    const box = document.getElementById("wb-apply-result");
    box.replaceChildren();
    box.hidden = false;
    const head = document.createElement("p"); head.className = "wb-plan-head";
    head.textContent = "Done — the upgrade is recorded in the project journal.";
    box.append(head);
    const outcome = result.outcome || {};
    if (outcome.note) { const n = document.createElement("p"); n.className = "muted"; n.textContent = outcome.note; box.append(n); }
    const steps = outcome.features || outcome.steps || {};
    const list = document.createElement("ul"); list.className = "wb-plan-steps";
    if (Array.isArray(outcome.files_changed) && outcome.files_changed.length) {
      const li = document.createElement("li"); li.textContent = `${outcome.files_changed.length} file(s) changed within the project.`; list.append(li);
    }
    box.append(list);
    // The project changed; the reviewed plan is consumed. Reload the detail
    // and re-show this confirmation once the refreshed state has rendered.
    document.getElementById("wb-apply").disabled = true;
    document.getElementById("wb-confirm").checked = false;
    document.getElementById("wb-confirm-wrap").hidden = true;
    loadWorkbenchDetail(workbench.id).finally(() => { box.hidden = false; });
  }

  async function planUpgrade() {
    clearWorkbenchNotice();
    resetPlanState();
    const feature = document.getElementById("wb-feature").value;
    const query = feature ? `?feature=${encodeURIComponent(feature)}` : "";
    document.getElementById("wb-plan").disabled = true;
    try {
      const result = await request(`/v1/admin/projects/${encodeURIComponent(workbench.id)}/plan${query}`, { headers: { Accept: "application/json" } });
      renderPlan(result);
    } catch (err) {
      showWorkbenchNotice(err.message || "The upgrade plan could not be produced. No files were changed.");
    } finally {
      document.getElementById("wb-plan").disabled = false;
    }
  }

  async function applyUpgrade() {
    clearWorkbenchNotice();
    if (!workbench.digest) { showWorkbenchNotice("Produce a plan before applying."); return; }
    if (!document.getElementById("wb-confirm").checked) { showWorkbenchNotice("Tick the confirmation box before applying — this writes to the project files."); return; }
    if (!workbench.idempotencyKey) {
      workbench.idempotencyKey = (window.crypto && crypto.randomUUID) ? crypto.randomUUID() : String(Date.now());
    }
    const feature = document.getElementById("wb-feature").value || null;
    document.getElementById("wb-apply").disabled = true;
    const { status, ok, body } = await requestStatus(`/v1/admin/projects/${encodeURIComponent(workbench.id)}/apply`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json", "Idempotency-Key": workbench.idempotencyKey },
      body: JSON.stringify({ confirm: true, plan_digest: workbench.digest, feature }),
    });
    document.getElementById("wb-apply").disabled = false;
    if (ok) { renderApplySuccess(body); return; }
    // An idempotent replay of the same apply returns a journaled operation
    // without re-running the write; surface it as accepted rather than an error.
    if (body && body.replay) { renderApplySuccess({ operation_id: body.operation_id || (body.operation && body.operation.op_id), outcome: { note: `Journaled operation state: ${body.state || (body.operation && body.operation.state) || "recorded"}.` } }); return; }
    renderApplyError(body, status);
  }

  // ---- Project lifecycle actions (generic, catalog-driven)
  //
  // Every executable catalog row — a `web` row whose `execution` block declares
  // a typed admin route and its parameter schema — renders a runnable
  // preview→confirm→apply control here, generated from the catalog rather than
  // a hard-wired per-command widget. The control builds one typed field per
  // declared parameter, so the browser can never send a shell command, argv
  // vector or filesystem path. The two-step flow mirrors the shipped
  // `feature add` gate: a preview returns a path-free descriptor plus its
  // digest and writes nothing; the confirmed run re-posts the same structured
  // fields with `confirm: true`, the exact `plan_digest` and an Idempotency-Key
  // header, delegating to the same in-process Core handler the CLI runs.
  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined && text !== null) node.textContent = text;
    return node;
  }

  // ---- Form error summaries (slice 3) ------------------------------------
  //
  // Every failed submission renders a focusable summary (heading + one link
  // per failing field) and keeps an inline error on each field, wired via
  // `aria-describedby`. Focus moves to the summary so screen-reader and
  // keyboard operators land on the recovery path. `target` is a container
  // id or the container element itself (workbench cards build theirs
  // dynamically); a missing container or field degrades to the retained
  // inline/banner rendering instead of throwing.
  function resolveSummary(target) {
    return typeof target === "string" ? document.getElementById(target) : target;
  }

  function renderErrorSummary(target, heading, items) {
    const container = resolveSummary(target);
    if (!container) return;
    container.replaceChildren();
    if (!items || !items.length) { container.hidden = true; return; }
    const title = document.createElement("h3");
    title.className = "error-summary-title";
    title.textContent = heading;
    container.append(title);
    const list = document.createElement("ul");
    list.className = "error-summary-list";
    for (const item of items) {
      const li = document.createElement("li");
      const field = item.fieldId ? document.getElementById(item.fieldId) : null;
      if (field) {
        const link = document.createElement("a");
        link.href = `#${item.fieldId}`;
        link.textContent = `${item.label}: ${item.message}`;
        link.addEventListener("click", (event) => {
          // Anchor navigation would fight the sticky topbar offset;
          // focus the field directly instead.
          event.preventDefault();
          const target = document.getElementById(item.fieldId);
          if (target) target.focus();
        });
        li.append(link);
      } else {
        li.textContent = item.label ? `${item.label}: ${item.message}` : item.message;
      }
      list.append(li);
    }
    container.append(list);
    container.hidden = false;
    try { container.focus({ preventScroll: true }); } catch (_) { /* summary already visible */ }
  }

  function clearErrorSummary(target) {
    const container = resolveSummary(target);
    if (container) { container.replaceChildren(); container.hidden = true; }
  }

  // Attach (or refresh) the inline error for one field: visible message,
  // `aria-invalid`, and the error id in `aria-describedby`. Clearing
  // removes the error id again so the describedby value returns to the
  // helper text alone.
  function setFieldError(fieldId, errorId, message) {
    const field = document.getElementById(fieldId);
    const inline = document.getElementById(errorId);
    if (inline) { inline.textContent = message; inline.hidden = false; }
    if (!field) return;
    field.setAttribute("aria-invalid", "true");
    const ids = (field.getAttribute("aria-describedby") || "").split(/\s+/).filter(Boolean);
    if (!ids.includes(errorId)) ids.push(errorId);
    if (ids.length) field.setAttribute("aria-describedby", ids.join(" "));
  }

  function clearFieldError(fieldId, errorId) {
    const field = document.getElementById(fieldId);
    const inline = document.getElementById(errorId);
    if (inline) { inline.textContent = ""; inline.hidden = true; }
    if (!field) return;
    field.removeAttribute("aria-invalid");
    const ids = (field.getAttribute("aria-describedby") || "").split(/\s+/).filter(Boolean).filter((id) => id !== errorId);
    if (ids.length) field.setAttribute("aria-describedby", ids.join(" "));
    else field.removeAttribute("aria-describedby");
  }

  function resetProjectActions() {
    const box = document.getElementById("wb-actions");
    if (box) box.replaceChildren();
  }

  // Summarize the path-free preview descriptor generically from its structured
  // fields (never a per-command template), so a newly-executable row renders
  // correctly without bespoke wiring. Long hexadecimal values (revisions,
  // digests-in-miniature) show as a short prefix: the exact value stays in
  // the confirmation the digest binds, never hand-copied by the operator.
  function shortValue(value) {
    let text;
    if (Array.isArray(value)) text = value.join(", ");
    else if (value !== null && typeof value === "object") text = JSON.stringify(value);
    else text = String(value);
    return text.replace(/[0-9a-f]{20,}/gi, (match) => `${match.slice(0, 12)}…`);
  }

  function summarizeDescriptor(descriptor) {
    const parts = [];
    for (const key of Object.keys(descriptor || {})) {
      if (key === "action" || key === "project_id") continue;
      parts.push(`${key}: ${shortValue(descriptor[key])}`);
    }
    const action = descriptor && descriptor.action ? descriptor.action : "action";
    return `${action} — ${parts.join("; ") || "no parameters"}`;
  }

  // Lifecycle stages, in the order an operator reads the page. Every
  // per-project action below carries an `lifecycle_stage` (one of these
  // keys) so the workbench can group buttons by what the operator is
  // trying to do, not by what the CLI command is named.
  const LIFECYCLE_STAGES = [
    { key: "adopt",      label: "Adopt",        help: "Bring the project into the registry or remove it from the fleet." },
    { key: "day-to-day", label: "Day-to-day",   help: "Work on the project: features, spec, upgrades, doctor, checker." },
    { key: "release",    label: "Release",      help: "Plan, apply, publish, deploy: the path from working to production." },
    { key: "retire",     label: "Retire",       help: "Deprecate, archive, or remove a project that is at the end of its life." },
  ];

  // Map catalog id prefixes to a lifecycle stage. The catalog already
  // groups every CLI command under one of these subcommands; this is
  // the same grouping the operator sees in `forge --help`, surfaced as
  // a button in the project view.
  function lifecycleStageFor(command) {
    if (!command || !command.id) return "day-to-day";
    const id = command.id;
    if (id.startsWith("new.") || id.startsWith("import.") || id.startsWith("register.") || id.startsWith("workspace.") || id.startsWith("graduation.")) return "adopt";
    if (id.startsWith("feature.") || id.startsWith("spec.") || id.startsWith("upgrade.") || id.startsWith("doctor.") || id.startsWith("kit.") || id.startsWith("test.") || id.startsWith("commit.")) return "day-to-day";
    if (id.startsWith("release.") || id.startsWith("deploy.") || id.startsWith("publish.") || id.startsWith("delivery.")) return "release";
    if (id.startsWith("retire") || id === "retire" || id.startsWith("deprecate") || id === "deprecate") return "retire";
    return "day-to-day";
  }

  function renderProjectActions() {
    const box = document.getElementById("wb-actions");
    if (!box) return;
    box.replaceChildren();
    if (!workbench.id) return;
    // Only rows addressed by the open project render here. The id-less
    // creation/registration rows are dashboard-level and render in
    // `renderManagementActions` instead.
    const rows = catalogCommands.filter(
      (command) => command.execution && command.execution.route.includes("{id}"),
    );
    if (!rows.length) {
      box.append(el("p", "muted", "No browser-executable actions are available for this project right now. Reload after the API and its catalog are reachable."));
      return;
    }
    // Group by lifecycle stage so the operator sees the buttons in the
    // order they would act on them: adopt, day-to-day, release, retire.
    const groups = new Map();
    for (const stage of LIFECYCLE_STAGES) groups.set(stage.key, []);
    for (const command of rows) groups.get(lifecycleStageFor(command)).push(command);
    for (const stage of LIFECYCLE_STAGES) {
      const group = groups.get(stage.key);
      if (!group || !group.length) continue;
      const wrap = el("div", "wb-actions-group");
      const head = el("h4", "wb-actions-head");
      head.textContent = stage.label;
      const help = el("p", "muted wb-actions-help");
      help.textContent = stage.help;
      wrap.append(head, help);
      const list = el("div", "wb-actions-list");
      for (const command of group) list.append(buildActionControl(command));
      wrap.append(list);
      box.append(wrap);
    }
  }

  // ---- Dashboard-level project management (creation / adoption) -----------
  //
  // The id-less executable rows (`forge new`, `forge import`,
  // `forge register`) resolve to routes with no `{id}` segment: the browser
  // supplies only a validated single-segment project name plus typed fields,
  // and the server joins that name to its own configured root. They render
  // here, once, outside any single project.
  function renderManagementActions() {
    const box = document.getElementById("management-actions");
    if (!box) return;
    box.replaceChildren();
    const rows = catalogCommands.filter(
      (command) => command.execution && !command.execution.route.includes("{id}"),
    );
    if (!rows.length) {
      box.append(el("p", "muted", "No project-management actions are available yet."));
      return;
    }
    for (const command of rows) {
      box.append(
        buildActionControl(command, {
          projectId: null,
          onSuccess: () => window.location.reload(),
        }),
      );
    }
  }

  // ---- Workspace onboarding (live discovery + bulk adopt) --------------
  //
  // The workspace root is server configuration, never a browser input: the
  // panel only ever sends validated single-segment directory leaves plus
  // typed `id`/`profile` overrides. Discovery re-reads the root on every
  // request, so an expanding workspace needs no other change. Selection is
  // previewed as one digest-bound batch, then confirmed; per-item results
  // are reported honestly and the fleet is reloaded on demand.
  const WS_CHUNK = 25;
  const ws = { candidates: [], digests: null, payload: null, discovered: false, autoSelected: null };
  // Project-scoped management state: when `?project=` is present the
  // management view leads with this single candidate (never the bulk table).
  const mgmtScoped = { param: null, candidate: null, digest: null, payload: null };

  function wsNotice(message) {
    const box = document.getElementById("ws-notice");
    box.textContent = message; box.hidden = false;
    document.getElementById("ws-error").hidden = true;
  }

  function wsError(message) {
    const box = document.getElementById("ws-error");
    box.textContent = message; box.hidden = false;
    document.getElementById("ws-notice").hidden = true;
  }

  function wsClear() {
    document.getElementById("ws-notice").hidden = true;
    document.getElementById("ws-error").hidden = true;
    clearErrorSummary("ws-error-summary");
  }

  function wsSelected() {
    const rows = document.querySelectorAll("#ws-rows tr");
    const items = [];
    for (const row of rows) {
      const tick = row.querySelector("input[type=checkbox]");
      if (!tick || !tick.checked) continue;
      const item = { directory: row.dataset.directory };
      const id = row.querySelector(".ws-id-override").value.trim();
      const profile = row.querySelector(".ws-profile-override").value.trim();
      if (id) item.id = id;
      if (profile) item.profile = profile;
      items.push(item);
    }
    return items;
  }

  function wsRefreshButtons() {
    const any = ws.candidates.some((c) => c.selectable);
    const chosen = wsSelected().length > 0;
    document.getElementById("ws-select-all").disabled = !any;
    document.getElementById("ws-preview").disabled = !chosen;
  }

  // ---- Management deep link (`/management?project=<identity>`) ------------
  //
  // Every fleet "Manage" row links here with its project identity, so a
  // detail visitor lands on a project-scoped onboarding card for that id —
  // never on the global bulk table as the primary content. The URL is the
  // source of truth: the web listener serves the shell for any query string
  // (reload safe), and `renderRoute` re-applies this on every `popstate`
  // (back/forward safe). An unknown or empty id selects nothing and says
  // so — it never selects the wrong project. The bulk table never ticks a
  // row for a deep link; hand ticks there are left alone.
  function managementProjectParam() {
    const id = (new URLSearchParams(window.location.search).get("project") || "").trim();
    return id === "" ? null : id;
  }

  function wsRowTick(directory) {
    return document.querySelector(`#ws-rows tr[data-directory="${CSS.escape(directory)}"] input[type="checkbox"]`);
  }

  function mgmtHasProjectParam() {
    return new URLSearchParams(window.location.search).has("project");
  }

  function mgmtScopedNotice(message) {
    const box = document.getElementById("mgmt-project-notice");
    if (!box) return;
    box.textContent = message;
    box.hidden = false;
  }

  function mgmtScopedClear() {
    const box = document.getElementById("mgmt-project-notice");
    if (box) { box.hidden = true; box.textContent = ""; }
    clearErrorSummary("mgmt-error-summary");
  }

  function mgmtScopedResetFlow() {
    mgmtScoped.digest = null;
    mgmtScoped.payload = null;
    const result = document.getElementById("mgmt-project-preview-result");
    if (result) { result.replaceChildren(); result.hidden = true; }
    const applyBox = document.getElementById("mgmt-project-apply-result");
    if (applyBox) { applyBox.replaceChildren(); applyBox.hidden = true; }
    const confirmWrap = document.getElementById("mgmt-project-confirm-wrap");
    if (confirmWrap) confirmWrap.hidden = true;
    const confirm = document.getElementById("mgmt-project-confirm");
    if (confirm) confirm.checked = false;
    const run = document.getElementById("mgmt-project-run");
    if (run) run.disabled = true;
  }

  function mgmtScopedSignal(candidate) {
    if (candidate.profile) return `${profileWords(candidate.profile)} (${candidate.confidence || "unknown"})`;
    if (candidate.manifest) return "manifest";
    return "—";
  }

  function mgmtScopedRender(candidate, id) {
    const title = document.getElementById("mgmt-project-title");
    const summary = document.getElementById("mgmt-project-summary");
    const detail = document.getElementById("mgmt-project-detail");
    const preview = document.getElementById("mgmt-project-preview");
    if (title) title.textContent = `Onboard ${candidate.directory || id} into Forge`;
    if (summary) {
      summary.textContent = candidate.selectable
        ? `Project-scoped onboarding for “${id}” — only this project's import/register actions are shown below. Nothing is written until you preview and confirm it.`
        : `Project “${id}” cannot be onboarded right now — nothing was selected.`;
    }
    if (detail) {
      detail.replaceChildren();
      detail.append(detailRow("Directory", candidate.directory || "—"));
      detail.append(detailRow("Project id", candidate.id || "—"));
      detail.append(detailRow("Action", candidate.action || "—"));
      detail.append(detailRow("Profile signal", mgmtScopedSignal(candidate)));
      const state = candidate.reason ? `${candidate.state}: ${candidate.reason}` : (candidate.state || "—");
      detail.append(detailRow("State", state));
    }
    if (preview) preview.disabled = !candidate.selectable;
  }

  async function mgmtScopedPreview() {
    mgmtScopedClear();
    const candidate = mgmtScoped.candidate;
    if (!candidate) {
      const message = "No project is selected — nothing was previewed.";
      mgmtScopedNotice(message);
      renderErrorSummary("mgmt-error-summary", "There is a problem previewing the project", [{ label: "Project", message }]);
      return;
    }
    const item = { directory: candidate.directory };
    const idOverride = (document.getElementById("mgmt-project-id") || {}).value || "";
    const profileOverride = (document.getElementById("mgmt-project-profile") || {}).value || "";
    if (idOverride.trim()) item.id = idOverride.trim();
    if (profileOverride.trim()) item.profile = profileOverride.trim();
    const box = document.getElementById("mgmt-project-preview-result");
    box.replaceChildren(); box.hidden = false;
    box.append(el("p", "wb-plan-head", "Preview — nothing has been written yet. Confirm to onboard exactly this project."));
    const list = el("ul", "wb-plan-steps");
    box.append(list);
    const { ok, body } = await wsPostChunk([item]);
    if (!ok) {
      const message = (body && body.error && body.error.message) || "The preview was refused. Nothing was written.";
      box.append(el("p", "wb-plan-head", message));
      renderErrorSummary("mgmt-error-summary", "The preview was refused", [{ label: "Project", message }]);
      mgmtScoped.digest = null; mgmtScoped.payload = null;
      document.getElementById("mgmt-project-run").disabled = true;
      return;
    }
    for (const plan of body.preview || []) {
      const line = plan.blocked
        ? `${plan.directory}: BLOCKED — ${plan.blocked}`
        : `${plan.directory}: ${plan.action} as ${plan.id} (${profileWords(plan.profile)})`;
      list.append(el("li", null, line));
    }
    mgmtScoped.digest = body.plan_digest;
    mgmtScoped.payload = [item];
    document.getElementById("mgmt-project-confirm-wrap").hidden = false;
    document.getElementById("mgmt-project-run").disabled = false;
  }

  async function mgmtScopedRun() {
    mgmtScopedClear();
    if (!mgmtScoped.payload || !mgmtScoped.digest) {
      const message = "Preview this project before running.";
      mgmtScopedNotice(message);
      renderErrorSummary("mgmt-error-summary", "There is a problem running the onboarding", [
        { fieldId: "mgmt-project-preview", label: "Preview", message },
      ]);
      return;
    }
    if (!document.getElementById("mgmt-project-confirm").checked) {
      const message = "Tick the confirmation box — this writes the manifest and registry row.";
      mgmtScopedNotice(message);
      renderErrorSummary("mgmt-error-summary", "There is a problem running the onboarding", [
        { fieldId: "mgmt-project-confirm", label: "Confirmation", message },
      ]);
      return;
    }
    document.getElementById("mgmt-project-run").disabled = true;
    const box = document.getElementById("mgmt-project-apply-result");
    box.replaceChildren(); box.hidden = false;
    const { ok, body } = await requestStatus("/v1/admin/workspace/onboard", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ items: mgmtScoped.payload, confirm: true, plan_digest: mgmtScoped.digest }),
    });
    const list = el("ul", "wb-plan-steps");
    if (!ok && !(body && body.results)) {
      const message = (body && body.error && body.error.message) || "The run was refused. Nothing was written.";
      box.append(el("p", "wb-plan-head", message));
      renderErrorSummary("mgmt-error-summary", "The onboarding run was refused", [{ label: "Project", message }]);
      if (body && body.preview && body.plan_digest) {
        box.append(el("p", "muted", "The selection changed: review a refreshed preview and confirm again."));
      }
      return;
    }
    for (const result of body.results || []) {
      list.append(el("li", null, result.ok
        ? `${result.directory}: onboarded as ${result.id}`
        : `${result.directory}: FAILED (${(result.code || "error")}) — ${result.message || "see journal"}`));
    }
    box.append(el("p", "wb-plan-head", `Onboarded ${body.succeeded ?? 0} of ${(body.succeeded ?? 0) + (body.failed ?? 0)} selected director${(body.succeeded ?? 0) + (body.failed ?? 0) === 1 ? "y" : "ies"}.`));
    box.append(list);
    mgmtScoped.digest = null; mgmtScoped.payload = null;
    document.getElementById("mgmt-project-confirm").checked = false;
    document.getElementById("mgmt-project-confirm-wrap").hidden = true;
    await wsDiscover();
    await wsReloadFleet();
  }

  function applyManagementProjectParam() {
    // Project-scoped vs bulk is decided by the presence of the `?project=`
    // key — not its value. A deep link always leads with the single-project
    // card and hides the bulk table, so a detail visitor never meets the
    // global "bring a workspace" panel as the primary content. Plain
    // `/management` (no key) keeps the bulk view exactly as before.
    const scoped = document.getElementById("mgmt-project");
    const bulk = document.getElementById("ws-bulk");
    const hasParam = mgmtHasProjectParam();
    if (scoped) scoped.hidden = !hasParam;
    if (bulk) bulk.hidden = !!hasParam;
    if (!hasParam) {
      // Leaving a deep link releases only its own suggestion; hand ticks stay.
      if (ws.autoSelected) {
        const prior = wsRowTick(ws.autoSelected.directory);
        if (prior) prior.checked = false;
        ws.autoSelected = null;
        wsRefreshButtons();
      }
      mgmtScoped.param = null;
      mgmtScoped.candidate = null;
      mgmtScopedResetFlow();
      mgmtScopedClear();
      return;
    }
    const id = managementProjectParam();
    // A present-but-empty `?project=` names nothing: scoped notice, no pick.
    if (!id) {
      mgmtScoped.param = "";
      mgmtScoped.candidate = null;
      mgmtScopedResetFlow();
      const title = document.getElementById("mgmt-project-title");
      if (title) title.textContent = "Project onboarding";
      const summary = document.getElementById("mgmt-project-summary");
      if (summary) summary.textContent = "No project id was given — nothing was selected.";
      const detail = document.getElementById("mgmt-project-detail");
      if (detail) detail.replaceChildren();
      const preview = document.getElementById("mgmt-project-preview");
      if (preview) preview.disabled = true;
      mgmtScopedNotice("No project id was given — nothing was selected.");
      if (ws.autoSelected) {
        const prior = wsRowTick(ws.autoSelected.directory);
        if (prior) prior.checked = false;
        ws.autoSelected = null;
        wsRefreshButtons();
      }
      return;
    }
    // Release the previous deep link's bulk suggestion (the scoped view
    // itself never ticks the bulk table).
    if (ws.autoSelected && ws.autoSelected.param !== id) {
      const prior = wsRowTick(ws.autoSelected.directory);
      if (prior) prior.checked = false;
      ws.autoSelected = null;
      wsRefreshButtons();
    }
    // Candidates arrive asynchronously via `wsDiscover`. Until the first
    // successful discovery there is nothing to match — show the scoped
    // loading state and leave the bulk table hidden.
    if (!ws.discovered) {
      const title = document.getElementById("mgmt-project-title");
      if (title) title.textContent = `Onboard ${id} into Forge`;
      const summary = document.getElementById("mgmt-project-summary");
      if (summary) summary.textContent = `Project-scoped onboarding for “${id}” — reading the workspace…`;
      mgmtScopedNotice(`Reading the workspace for “${id}”…`);
      return;
    }
    // Already rendered for this URL — respect whatever the operator did
    // since (including a scoped preview) instead of re-rendering it.
    if (mgmtScoped.param === id && mgmtScoped.candidate) return;
    mgmtScoped.param = id;
    mgmtScoped.candidate = null;
    mgmtScopedResetFlow();
    const match = ws.candidates.find((c) => c.id === id) || ws.candidates.find((c) => c.directory === id);
    if (!match) {
      const title = document.getElementById("mgmt-project-title");
      if (title) title.textContent = "Project onboarding";
      const summary = document.getElementById("mgmt-project-summary");
      if (summary) summary.textContent = "No matching project — nothing was selected.";
      const detail = document.getElementById("mgmt-project-detail");
      if (detail) detail.replaceChildren();
      const preview = document.getElementById("mgmt-project-preview");
      if (preview) preview.disabled = true;
      mgmtScopedNotice(`No onboardable workspace entry matches “${id}” — nothing was selected.`);
      return;
    }
    if (match.state === "registered") {
      // Already managed: the onboarding panels cannot manage this
      // project, but the workbench can — land there instead of reporting
      // that it cannot be onboarded. The workbench never bounces back for
      // a managed id, so this cannot loop. `replaceState` (not a push) so
      // Back returns past this redirect instead of re-triggering it.
      window.history.replaceState(null, "", `/workbench?project=${encodeURIComponent(id)}`);
      renderRoute();
      return;
    }
    if (!match.selectable) {
      mgmtScoped.candidate = null;
      mgmtScopedRender({ ...match, selectable: false }, id);
      const why = match.state ? ` (${match.state}${match.reason ? `: ${match.reason}` : ""})` : "";
      mgmtScopedNotice(`${match.id || match.directory} cannot be onboarded right now${why} — nothing was selected.`);
      return;
    }
    mgmtScoped.candidate = match;
    mgmtScopedRender(match, id);
    mgmtScopedNotice(`Selected ${match.directory} for onboarding — review the preview below, then confirm. Nothing is written until you confirm it.`);
  }

  function renderWsFleetHint(candidates) {
    const hint = document.getElementById("ws-fleet-hint");
    if (!hint) return;
    if (!candidates) { hint.hidden = true; hint.replaceChildren(); return; }
    const onboardable = candidates.filter((c) => c.selectable).length;
    hint.replaceChildren();
    if (!onboardable) { hint.hidden = true; return; }
    hint.hidden = false;
    hint.append(
      document.createTextNode(`${onboardable} of ${candidates.length} workspace directories are not yet onboarded — `),
    );
    const link = document.createElement("a");
    link.href = "/management";
    link.textContent = "open Workspace onboarding";
    hint.append(link);
  }

  async function wsDiscover() {
    wsClear();
    ws.digests = null; ws.payload = null;
    document.getElementById("ws-preview-result").hidden = true;
    document.getElementById("ws-confirm-wrap").hidden = true;
    document.getElementById("ws-confirm").checked = false;
    document.getElementById("ws-run").disabled = true;
    wsNotice("Reading the workspace…");
    let data;
    try {
      data = await request("/v1/admin/workspace/candidates?limit=100", { headers: { Accept: "application/json" } });
    } catch (err) {
      renderWsFleetHint(null);
      if (err && err.status === 409) {
        wsNotice("Workspace onboarding needs FORGE_ADMIN_PROJECTS_ROOT set in the API server environment — see README “Running the web portal”. Single-project controls above keep working.");
      } else {
        wsError(err.message || "Workspace discovery is unavailable right now.");
      }
      return;
    }
    ws.candidates = data.candidates || [];
    const body = document.getElementById("ws-rows");
    body.replaceChildren();
    for (const candidate of ws.candidates) {
      const row = document.createElement("tr");
      row.dataset.directory = candidate.directory;
      const tickCell = document.createElement("td");
      const tick = document.createElement("input");
      tick.type = "checkbox";
      tick.disabled = !candidate.selectable;
      tick.setAttribute("aria-label", `Onboard ${candidate.directory}`);
      tick.addEventListener("change", wsRefreshButtons);
      tickCell.append(tick);
      row.append(tickCell, textCell(candidate.directory));
      const idCell = textCell(candidate.id || "—");
      row.append(idCell);
      row.append(textCell(candidate.action || "—"));
      const signal = candidate.profile
        ? `${profileWords(candidate.profile)} (${candidate.confidence || "unknown"})`
        : (candidate.manifest ? "manifest" : "—");
      row.append(textCell(signal));
      const state = candidate.reason
        ? `${candidate.state}: ${candidate.reason}`
        : candidate.state;
      row.append(textCell(state));
      const idOverride = document.createElement("td");
      const idField = document.createElement("input");
      idField.type = "text"; idField.autocomplete = "off";
      idField.placeholder = "id override"; idField.className = "ws-id-override";
      idField.setAttribute("aria-label", `Id override for ${candidate.directory}`);
      idField.disabled = !candidate.selectable;
      idOverride.append(idField);
      row.append(idOverride);
      const profileOverride = document.createElement("td");
      const profileField = document.createElement("input");
      profileField.type = "text"; profileField.autocomplete = "off";
      profileField.placeholder = "profile override"; profileField.className = "ws-profile-override";
      profileField.setAttribute("aria-label", `Profile override for ${candidate.directory}`);
      profileField.disabled = !candidate.selectable;
      profileOverride.append(profileField);
      row.append(profileOverride);
      body.append(row);
    }
    const onboardable = ws.candidates.filter((c) => c.selectable).length;
    document.getElementById("ws-table-wrap").hidden = ws.candidates.length === 0;
    document.getElementById("ws-empty").hidden = ws.candidates.length > 0;
    wsNotice(`Found ${ws.candidates.length} directories (${onboardable} onboardable) of ${data.total} total. New siblings appear here on Refresh.`);
    renderWsFleetHint(ws.candidates);
    wsRefreshButtons();
    ws.discovered = true;
    // A `?project=` deep link (or a reload carrying one) can only be matched
    // once this table exists; the call is a no-op without the parameter.
    applyManagementProjectParam();
  }

  function wsSelectAll() {
    for (const tick of document.querySelectorAll("#ws-rows input[type=checkbox]")) {
      if (!tick.disabled) tick.checked = true;
    }
    wsRefreshButtons();
  }

  async function wsPostChunk(items) {
    return requestStatus("/v1/admin/workspace/onboard", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ items }),
    });
  }

  async function wsPreview() {
    wsClear();
    const items = wsSelected();
    if (!items.length) {
      const message = "Tick at least one onboardable directory first.";
      wsError(message);
      renderErrorSummary("ws-error-summary", "There is a problem previewing the selection", [
        { fieldId: "ws-select-all", label: "Selection", message },
      ]);
      return;
    }
    const chunks = [];
    for (let i = 0; i < items.length; i += WS_CHUNK) chunks.push(items.slice(i, i + WS_CHUNK));
    const box = document.getElementById("ws-preview-result");
    box.replaceChildren(); box.hidden = false;
    box.append(el("p", "wb-plan-head", "Preview — nothing has been written yet. Confirm to onboard exactly this selection."));
    const list = el("ul", "wb-plan-steps");
    box.append(list);
    const digests = [];
    for (const [index, chunk] of chunks.entries()) {
      const { ok, body } = await wsPostChunk(chunk);
      if (!ok) {
        const message = (body && body.error && body.error.message) || `Batch ${index + 1} was refused. Nothing was written.`;
        box.append(el("p", "wb-plan-head", message));
        renderErrorSummary("ws-error-summary", "The preview was refused", [{ label: `Batch ${index + 1}`, message }]);
        ws.digests = null; ws.payload = null;
        document.getElementById("ws-run").disabled = true;
        return;
      }
      for (const plan of body.preview || []) {
        const line = plan.blocked
          ? `${plan.directory}: BLOCKED — ${plan.blocked}`
          : `${plan.directory}: ${plan.action} as ${plan.id} (${profileWords(plan.profile)})`;
        list.append(el("li", null, line));
      }
      // The batch digest stays in memory for the confirmed run; it is
      // never shown because operators confirm the listed items, not hashes.
      digests.push({ items: chunk, digest: body.plan_digest });
    }
    ws.digests = digests;
    ws.payload = { chunks: digests.map((d) => d.items) };
    document.getElementById("ws-confirm-wrap").hidden = false;
    document.getElementById("ws-run").disabled = false;
  }

  // Re-read the fleet after a successful onboard and re-render rows plus
  // counts in place. The onboarding results panel stays visible; a failed
  // refresh keeps the manual Reload control as the fallback path.
  async function wsReloadFleet() {
    let data;
    try {
      data = await request("/v1/admin/projects", { headers: { Accept: "application/json" } });
    } catch (_) {
      return false;
    }
    const projects = data.projects || [];
    window.__forgeProjects = projects;
    renderSources(data.sources || []);
    document.getElementById("summary-total").textContent = String(data.summary?.registered ?? 0);
    document.getElementById("summary-profiles").textContent = String(data.summary?.profiles ?? new Set(projects.map((project) => project.profile)).size);
    document.getElementById("summary-evidence").textContent = String(data.summary?.with_evidence ?? 0);
    document.getElementById("updated-label").textContent = `Updated ${new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
    renderProjects(projects);
    const select = document.getElementById("workbench-project");
    if (select) {
      const current = select.value;
      select.replaceChildren();
      const placeholder = document.createElement("option"); placeholder.value = ""; placeholder.textContent = "Choose a managed project…"; select.append(placeholder);
      for (const project of managedProjects(projects)) {
        const option = document.createElement("option"); option.value = project.identity; option.textContent = project.name || project.identity; select.append(option);
      }
      if (current) select.value = current;
    }
    populateDeliveryProjects(projects);
    loadFleetReadiness();
    return true;
  }

  async function wsRun() {
    wsClear();
    if (!ws.payload || !ws.digests || !ws.digests.length) {
      const message = "Preview the selection before running.";
      wsError(message);
      renderErrorSummary("ws-error-summary", "There is a problem running the onboarding", [
        { fieldId: "ws-preview", label: "Preview", message },
      ]);
      return;
    }
    if (!document.getElementById("ws-confirm").checked) {
      const message = "Tick the confirmation box — this writes manifests and registry rows.";
      wsError(message);
      renderErrorSummary("ws-error-summary", "There is a problem running the onboarding", [
        { fieldId: "ws-confirm", label: "Confirmation", message },
      ]);
      return;
    }
    document.getElementById("ws-run").disabled = true;
    const box = document.getElementById("ws-apply-result");
    box.replaceChildren(); box.hidden = false;
    let succeeded = 0;
    let failed = 0;
    const list = el("ul", "wb-plan-steps");
    box.append(list);
    for (const [index, chunk] of ws.digests.entries()) {
      const { ok, body } = await requestStatus("/v1/admin/workspace/onboard", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ items: chunk.items, confirm: true, plan_digest: chunk.digest }),
      });
      if (!ok && !(body && body.results)) {
        const message = `Batch ${index + 1} refused: ${(body && body.error && body.error.message) || "nothing was written."}`;
        list.append(el("li", null, message));
        renderErrorSummary("ws-error-summary", "The onboarding run was refused", [{ label: `Batch ${index + 1}`, message }]);
        if (body && body.preview && body.plan_digest) {
          list.append(el("li", null, "The selection changed: review a refreshed preview and confirm again."));
        }
        failed += chunk.items.length;
        break;
      }
      for (const result of body.results || []) {
        list.append(el("li", null, result.ok
          ? `${result.directory}: onboarded as ${result.id}`
          : `${result.directory}: FAILED (${(result.code || "error")}) — ${result.message || "see journal"}`));
      }
      succeeded += body.succeeded ?? 0;
      failed += body.failed ?? 0;
    }
    box.prepend(el("p", "wb-plan-head", `Onboarded ${succeeded} of ${succeeded + failed} selected directories${failed ? ` — ${failed} failed, see above` : ""}.`));
    ws.digests = null; ws.payload = null;
    document.getElementById("ws-confirm").checked = false;
    document.getElementById("ws-confirm-wrap").hidden = true;
    document.getElementById("ws-reload").hidden = false;
    // Refresh the workspace table, then the fleet in place — results stay
    // visible either way and Reload remains the manual fallback.
    await wsDiscover();
    if (!(await wsReloadFleet())) {
      box.append(el("p", "muted", "The project list could not refresh itself — use Reload project list."));
    }
  }

  function initWorkspaceOnboarding() {
    document.getElementById("ws-discover").addEventListener("click", wsDiscover);
    document.getElementById("ws-select-all").addEventListener("click", wsSelectAll);
    document.getElementById("ws-preview").addEventListener("click", wsPreview);
    document.getElementById("ws-run").addEventListener("click", wsRun);
    document.getElementById("ws-reload").addEventListener("click", () => window.location.reload());
    document.getElementById("mgmt-project-preview").addEventListener("click", mgmtScopedPreview);
    document.getElementById("mgmt-project-run").addEventListener("click", mgmtScopedRun);
  }

  function buildActionControl(command, scope = {}) {
    const execution = command.execution;
    const method = (execution.method || "POST").toUpperCase();
    const scoped = execution.route.includes("{id}");
    const projectId = scope.projectId === undefined ? workbench.id : scope.projectId;
    const card = el("div", "wb-action-card");
    card.dataset.command = command.id;

    // One open form at a time. A wall of pre-expanded forms is the reason the
    // workbench read as unhuman: twelve stacked inputs, most of them
    // irrelevant to the project's current state, with no indication which
    // action matters now. Collapsed to a row, the operator sees a list of
    // choices and opens the one they want.
    const head = el("button", "wb-action-head");
    head.type = "button";
    head.setAttribute("aria-expanded", "false");
    const caret = el("span", "wb-action-caret", "▸");
    caret.setAttribute("aria-hidden", "true");
    const headText = el("span", "wb-action-headtext");
    // The human label leads; the CLI invocation is secondary, for the operator
    // who wants to reproduce the action in a terminal. It is never the title.
    //
    // `command.label` is only the leaf subcommand, so three different rows read
    // as the same word: "apply" for `spec apply`, `release apply` and
    // `deploy apply`. The catalog summary already opens with a distinct human
    // phrase for each ("Apply a verified release plan", "Apply a routing
    // decision"), so the title is that phrase, falling back to the label only
    // when the summary has no leading clause to borrow.
    headText.append(el("span", "wb-action-label", humanActionTitle(command)));
    headText.append(el("span", "wb-action-cli", `forge ${command.id.replace(/\./g, " ")}`));
    head.append(caret, headText);
    card.append(head);

    const body = el("div", "wb-action-body");
    body.hidden = true;
    card.append(body);
    body.append(el("p", "muted wb-action-summary", command.summary));

    // One typed control per declared parameter (a closed scalar kind maps to a
    // matching input); there is deliberately no free-text command/path/argv.
    // Staged delivery confirmations arrive pre-filled from the project's
    // delivery status, so the operator reviews values instead of copying
    // operation numbers or revisions by hand.
    const inputs = [];
    const tools = el("div", "wb-plan-tools");
    const prefill = (workbench.delivery && typeof workbench.delivery === "object")
      ? workbench.delivery
      : {};
    for (const param of execution.parameters || []) {
      // A stacked field (label above input), not the topbar's inline
      // `search-box`: reusing that class collapsed the label into the input
      // row at the wrong size. The label text is visible and persistent;
      // required fields carry a visible marker.
      const label = el("label", "wb-field");
      const fieldId = `wb-${String(command.id).replace(/[^a-zA-Z0-9_-]/g, "-")}-${param.name}`;
      const errorId = `${fieldId}-error`;
      let field;
      if (param.kind === "boolean") {
        field = el("input"); field.type = "checkbox"; field.checked = false;
        field.id = fieldId;
        const visible = el("span", "wb-field-label", `${paramLabel(param)}${param.required ? " *" : ""}`);
        label.append(visible);
      } else {
        field = el("input"); field.type = "text"; field.autocomplete = "off";
        field.id = fieldId;
        // Say what to type, not just the field name. A field labelled only
        // `feature` gives an operator nothing to act on.
        field.placeholder = paramHint(param);
        const visible = el("span", "wb-field-label", `${paramLabel(param)}${param.required ? " *" : ""}`);
        label.append(visible);
        const known = prefill[param.name];
        if ((typeof known === "string" && known) || typeof known === "number") {
          field.value = String(known);
        }
      }
      // Announced to assistive technology; no native validation UI fires
      // because these inputs never submit a form.
      if (param.required) field.required = true;
      field.setAttribute("aria-describedby", errorId);
      const inline = el("span", "field-error");
      inline.id = errorId;
      inline.hidden = true;
      label.append(field, inline);
      tools.append(label);
      inputs.push({ param, field, fieldId, errorId });
    }
    if (inputs.length) body.append(tools);

    // Card-local error summary: heading plus one link per failing field.
    // Focus moves here on a failed preview or refused run; the existing
    // result rendering below is retained.
    const summaryBox = el("div", "error-summary");
    summaryBox.tabIndex = -1;
    summaryBox.hidden = true;
    body.append(summaryBox);

    function clearCardErrors() {
      clearErrorSummary(summaryBox);
      for (const { fieldId, errorId } of inputs) clearFieldError(fieldId, errorId);
    }

    const state = { digest: null, payload: null, idempotencyKey: null };
    const result = el("div", "wb-plan-result"); result.hidden = true;
    const confirmWrap = el("label", "wb-confirm");
    const confirmBox = el("input"); confirmBox.type = "checkbox";
    confirmWrap.append(confirmBox, document.createTextNode(" I confirm this will write to the project."));
    confirmWrap.hidden = true;
    const runWrap = el("div", "wb-plan-tools");
    const previewBtn = el("button", "button button-quiet"); previewBtn.type = "button";
    previewBtn.textContent = `Preview ${command.label}`;
    const runBtn = el("button", "button"); runBtn.type = "button"; runBtn.disabled = true;
    runBtn.textContent = "Run confirmed action";
    runWrap.append(previewBtn, runBtn);

    // Resolve the row's declared route to a concrete path for the open project;
    // the browser only ever substitutes the validated id, never a path it typed.
    const routePath = () => {
      const template = execution.route.split(" ").slice(1).join(" ");
      return template.replace("{id}", encodeURIComponent(projectId || ""));
    };

    const gatherPayload = () => {
      const payload = {};
      const failures = [];
      for (const { param, field, fieldId, errorId } of inputs) {
        if (param.kind === "boolean") {
          if (field.checked) payload[param.name] = true;
        } else if (param.kind === "string_array") {
          const values = field.value.split(",").map((v) => v.trim()).filter(Boolean);
          if (param.required && !values.length) { failures.push({ fieldId, errorId, label: paramLabel(param), message: `Enter at least one ${param.name}.` }); continue; }
          if (values.length) payload[param.name] = values;
        } else {
          const value = field.value.trim();
          if (param.required && !value) { failures.push({ fieldId, errorId, label: paramLabel(param), message: `Enter a ${param.name}.` }); continue; }
          if (value) payload[param.name] = value;
        }
      }
      if (failures.length) {
        clearCardErrors();
        showWorkbenchNotice(failures[0].message);
        for (const failure of failures) setFieldError(failure.fieldId, failure.errorId, failure.message);
        renderErrorSummary(summaryBox, "There is a problem with this action", failures);
        return null;
      }
      return payload;
    };

    const showPreview = (body) => {
      result.replaceChildren(); result.hidden = false;
      result.append(el("p", "wb-plan-head", "Preview — nothing has been written yet. Confirm to run this exact action."));
      result.append(el("p", "wb-plan-steps", summarizeDescriptor(body.preview)));
      // The digest stays in memory for the confirmed run; operators review
      // the listed action, never the hash.
      state.digest = body.plan_digest;
      confirmWrap.hidden = false;
      runBtn.disabled = false;
    };
    const showError = (body, status) => {
      result.replaceChildren(); result.hidden = false;
      const message = (body && body.error && body.error.message) || "The action was refused. Nothing was written.";
      result.append(el("p", "wb-plan-head", message));
      renderErrorSummary(summaryBox, "The action was refused", [{ label: humanActionTitle(command), message }]);
      // A stale digest (409) returns a refreshed preview so the operator can
      // re-review and re-confirm rather than acting blind.
      if (body && body.preview && body.plan_digest) showPreview(body);
      else if (status === 409) runBtn.disabled = true;
    };
    const showSuccess = (body) => {
      const message = "Done — recorded in the project journal.";
      clearCardErrors();
      // Drop the reviewed digest and confirmation controls so the action cannot
      // be blindly re-armed, then refresh the workbench to show the new state.
      state.digest = null; state.payload = null; state.idempotencyKey = null;
      confirmWrap.hidden = true; confirmBox.checked = false; runBtn.disabled = true;
      const show = () => {
        result.replaceChildren(); result.hidden = false;
        result.append(el("p", "wb-plan-head", message));
      };
      if (scope.onSuccess) { scope.onSuccess(); show(); return; }
      if (workbench.id) {
        loadWorkbenchDetail(workbench.id)
          .then(() => loadProjectDelivery(workbench.id))
          .then(show)
          .catch(show);
        return;
      }
      show();
    };

    previewBtn.addEventListener("click", async () => {
      if (scoped && !projectId) { showWorkbenchNotice("Open a managed project first."); return; }
      clearWorkbenchNotice();
      clearCardErrors();
      result.hidden = true; result.replaceChildren();
      confirmWrap.hidden = true; confirmBox.checked = false; runBtn.disabled = true;
      const payload = gatherPayload();
      if (!payload) return;
      state.payload = payload;
      const { ok, body } = await requestStatus(routePath(), {
        method,
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(payload),
      });
      if (ok) showPreview(body); else showError(body, undefined);
    });

    runBtn.addEventListener("click", async () => {
      if (!state.payload || !state.digest) { showWorkbenchNotice("Produce a preview before running."); return; }
      if (!confirmBox.checked) { showWorkbenchNotice("Tick the confirmation box — this writes to the project."); return; }
      if (!state.idempotencyKey) {
        state.idempotencyKey = (window.crypto && crypto.randomUUID) ? crypto.randomUUID() : String(Date.now());
      }
      runBtn.disabled = true;
      const body = { ...state.payload, confirm: true, plan_digest: state.digest };
      const { status, ok, body: applied } = await requestStatus(routePath(), {
        method,
        headers: { "Content-Type": "application/json", Accept: "application/json", "Idempotency-Key": state.idempotencyKey },
        body: JSON.stringify(body),
      });
      runBtn.disabled = false;
      if (ok) { state.idempotencyKey = null; showSuccess(applied); return; }
      if (applied && applied.replay) { state.idempotencyKey = null; showSuccess({ operation_id: applied.operation_id }); return; }
      state.idempotencyKey = null;
      showError(applied, status);
    });

    body.append(confirmWrap, runWrap, result);

    // Opening one action closes the others. An operator working through a
    // project sees one form, not twelve.
    head.addEventListener("click", () => {
      const opening = body.hidden;
      for (const other of document.querySelectorAll(".wb-action-card[data-command]")) {
        if (other === card) continue;
        other.querySelector(".wb-action-body").hidden = true;
        other.querySelector(".wb-action-head").setAttribute("aria-expanded", "false");
        other.classList.remove("is-open");
      }
      body.hidden = !opening;
      head.setAttribute("aria-expanded", String(opening));
      card.classList.toggle("is-open", opening);
      if (opening && !inputs.length) previewBtn.focus();
    });
    card.classList.add("is-collapsed");
    return card;
  }

  // The catalog's `label` is only the leaf subcommand, so `spec apply`,
  // `release apply` and `deploy apply` all read as the single word "apply".
  // Each summary already opens with a distinct human phrase, so the title
  // borrows that clause and falls back to the label when there is none to
  // borrow. Derived from the catalog contract, not hardcoded per command.
  function humanActionTitle(command) {
    const summary = (command.summary || "").trim();
    if (summary) {
      // Split on the separators the catalog summaries actually use. Sourcing
      // these from the summaries keeps the title distinct per action without a
      // per-command table that would rot on the next command added.
      const clause = summary.split(/[:;—]/)[0].trim();
      if (clause && clause.length <= 96) return clause.replace(/[.,]$/, "");
    }
    // Two `delivery` rows share the clause prefix "Invoke the provider's
    // `publish` for …", so a long clause is not just verbose but ambiguous.
    // Falling back to the command family keeps them distinguishable without the
    // browser inventing per-command prose it cannot keep honest.
    if (command.parent_id) return `${command.parent_id} · ${command.label || command.id}`;
    return command.label || command.id;
  }

  // A field labelled only by its parameter name (`feature`, `version`,
  // `operation_id`) gives an operator nothing to act on. These labels and
  // placeholders say what to type, in the operator's words.
  const PARAM_LABELS = {
    feature: "Feature name",
    version: "Version",
    spec: "Spec name",
    decision: "Routing decision",
    project: "Project name",
    id: "Project id",
    profile: "Profile",
    operation_id: "Operation id",
    reason: "Reason",
    target: "Target environment",
    proposal: "Proposal id",
  };
  const PARAM_HINTS = {
    feature: "e.g. auth",
    version: "e.g. 2.1",
    spec: "e.g. auth-refactor",
    decision: "deterministic, semantic, or manual",
    project: "the project's directory name",
    id: "lowercase-with-dashes",
    profile: "e.g. rust-web",
    operation_id: "copied from the delivery status above",
    reason: "why this is needed",
    target: "staging or production",
    proposal: "e.g. description-a1b2c3d4",
  };
  function paramLabel(param) {
    if (PARAM_LABELS[param.name]) return PARAM_LABELS[param.name];
    return param.name.replace(/_/g, " ").replace(/^\w/, (c) => c.toUpperCase());
  }
  function paramHint(param) {
    if (param.kind === "string_array") {
      return `${paramLabel(param).toLowerCase()}, comma-separated`;
    }
    return PARAM_HINTS[param.name] || param.name.replace(/_/g, " ");
  }

  function initWorkbench(projects) {
    const select = document.getElementById("workbench-project");
    select.replaceChildren();
    const placeholder = document.createElement("option"); placeholder.value = ""; placeholder.textContent = "Choose a managed project…"; select.append(placeholder);
    for (const project of managedProjects(projects)) {
      const option = document.createElement("option"); option.value = project.identity; option.textContent = project.name || project.identity; select.append(option);
    }
    document.getElementById("workbench-load").addEventListener("click", () => {
      const id = select.value;
      if (!id) { showWorkbenchNotice("Select a managed project first."); return; }
      // Route through the URL so the open project stays deep-linkable;
      // the route render applies the load exactly once.
      navigateTo(`/workbench?project=${encodeURIComponent(id)}`);
    });
    select.addEventListener("change", () => {
      if (select.value) navigateTo(`/workbench?project=${encodeURIComponent(select.value)}`);
      else { document.getElementById("workbench-body").hidden = true; document.getElementById("workbench-empty").hidden = false; clearWorkbenchNotice(); }
    });
    document.getElementById("wb-plan").addEventListener("click", planUpgrade);
    document.getElementById("wb-apply").addEventListener("click", applyUpgrade);
    document.getElementById("wb-maintain-refresh").addEventListener("click", () => {
      if (!workbench.id) { showWorkbenchNotice("Select a managed project first."); return; }
      loadMaintain(workbench.id);
    });
    // A `?project=` deep link (or a reload carrying one) can only be matched
    // once this selector exists; the call is a no-op without the parameter.
    wbFleetReady = true;
    applyWorkbenchProjectParam();
  }

  // ---- Workbench deep link (`/workbench?project=<identity>`) --------------
  //
  // Every managed fleet row's "Open" action navigates here with its project
  // identity, so the operator lands on the workbench with that project
  // selected and loaded. The URL is the source of truth when it names a
  // project: boot, reload and back/forward all reconcile through
  // `renderRoute`. A bare `/workbench` never auto-loads and never disturbs
  // a hand-made selection; an unknown id loads nothing and says so.
  function workbenchProjectParam() {
    const id = (new URLSearchParams(window.location.search).get("project") || "").trim();
    return id === "" ? null : id;
  }

  function applyWorkbenchProjectParam() {
    const select = document.getElementById("workbench-project");
    // The fleet has not populated the selector yet (direct deep-link load
    // races the async fleet fetch): `initWorkbench` applies the parameter
    // once the options exist.
    if (!select || !wbFleetReady) return;
    const id = workbenchProjectParam();
    // No (or empty) parameter: a plain `/workbench` visit changes nothing —
    // whatever the operator opened by hand stays open.
    if (!id) return;
    // Already showing (or already loading) this project: respect whatever
    // the operator did since instead of re-fetching the detail.
    if (workbench.id === id || wbAutoParam === id) return;
    const known = Array.from(select.options).some((option) => option.value === id);
    if (!known) {
      wbAutoParam = null;
      showWorkbenchNotice(`No managed project matches “${id}” — nothing was loaded.`);
      return;
    }
    select.value = id;
    wbAutoParam = id;
    loadWorkbenchDetail(id);
  }

  // ---- Portfolio controls
  // Forge-owned metadata across projects plus a truthful cross-project
  // evidence surface. Every value is rendered as text via textContent —
  // never raw-HTML assignment — and the only writes are typed metadata
  // mutations that send an id and fixed fields. Imported evidence is shown
  // read-only; the browser sends no path, command text or provider probe.
  const PORTFOLIO_STATUS_LABELS = {
    fresh: "Fresh", stale: "Stale", unconfigured: "Not configured", unavailable: "Unavailable",
    available: "Available", not_run: "Not run", "not-run": "Not run", not_ready: "Not ready",
    "not-ready": "Not ready", ready: "Ready", disabled: "Disabled", supported: "Supported",
    planned: "Planned", enabled: "Enabled", observed: "Observed", valid: "Valid",
    invalid: "Invalid", no_projects: "No projects", evaluated: "Evaluated",
  };
  const PORTFOLIO_CONFIDENCE = ["unknown", "low", "medium", "high"];

  function statusLabel(state) {
    return PORTFOLIO_STATUS_LABELS[state] || (state || "unknown");
  }

  function showPortfolioNotice(message) {
    const notice = document.getElementById("portfolio-notice");
    notice.textContent = message;
    notice.hidden = false;
  }

  function clearPortfolioNotice() {
    const notice = document.getElementById("portfolio-notice");
    notice.hidden = true;
    notice.textContent = "";
  }

  function showPortfolioError(message) {
    const error = document.getElementById("portfolio-error");
    error.textContent = message;
    error.hidden = false;
  }

  function clearPortfolioError() {
    const error = document.getElementById("portfolio-error");
    error.hidden = true;
    error.textContent = "";
  }

  function portfolioSourceItem(label, state) {
    const li = document.createElement("li");
    li.className = "source-item";
    const name = document.createElement("span");
    name.textContent = label;
    const badge = makeBadge(statusLabel(state), "state-observed");
    li.append(name, badge);
    return li;
  }

  function renderPortfolioEvidence(sections) {
    const list = document.getElementById("portfolio-sources");
    list.textContent = "";
    document.getElementById("portfolio-evidence-empty").hidden = !!sections;
    if (!sections) return;
    const fleet = sections.fleet || {};
    (fleet.sources || []).forEach((source) => {
      list.append(portfolioSourceItem(`Fleet · ${source.name || source.source || "source"}`, source.state));
    });
    const provider = sections.provider || {};
    (provider.rows || []).forEach((row) => {
      list.append(portfolioSourceItem(`Provider · ${row.provider}`, row.status));
    });
    list.append(portfolioSourceItem("Readiness", (sections.readiness || {}).state || "not_run"));
    (sections.governance ? sections.governance.providers || [] : []).forEach((row) => {
      list.append(portfolioSourceItem(`Governance · ${row.provider}`, row.status));
    });
    (sections.analytics ? sections.analytics.providers || [] : []).forEach((row) => {
      list.append(portfolioSourceItem(`Analytics · ${row.provider}`, row.support));
    });
    const gaps = sections.gaps || {};
    list.append(portfolioSourceItem(`Gap report · ${gaps.total ?? 0} findings`, gaps.clean ? "ready" : "not_ready"));
    const interest = sections.interest || {};
    const withheld = (interest.verdicts || []).filter((v) => v.withheld).length;
    list.append(portfolioSourceItem(`Interest · ${withheld} withheld`, interest.state || "unavailable"));
  }

  function renderPortfolioProjects(projects) {
    const tbody = document.getElementById("portfolio-rows");
    tbody.textContent = "";
    document.getElementById("portfolio-project-empty").hidden = projects.length > 0;
    projects.forEach((project) => {
      const row = document.createElement("tr");
      const name = document.createElement("td");
      name.textContent = project.project_id || "—";
      const tags = (project.tags || []).join(", ");
      row.append(
        name,
        textCell(project.lifecycle),
        textCell(project.confidence),
        textCell(tags),
        textCell(project.evidence_summary),
        textCell(project.next_action),
      );
      tbody.append(row);
    });
  }

  function populatePortfolioProjects(projects) {
    const select = document.getElementById("portfolio-project");
    select.textContent = "";
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Choose a project…";
    select.append(placeholder);
    projects.forEach((project) => {
      const option = document.createElement("option");
      option.value = project.project_id;
      option.textContent = project.project_id;
      select.append(option);
    });
  }

  async function loadPortfolio() {
    clearPortfolioError();
    try {
      const data = await request("/v1/admin/portfolio", { headers: { Accept: "application/json" } });
      const projects = data.projects || [];
      renderPortfolioProjects(projects);
      populatePortfolioProjects(projects);
    } catch (err) {
      showPortfolioError(err.message || "Portfolio metadata unavailable.");
    }
    try {
      const evidence = await request("/v1/admin/portfolio/evidence", { headers: { Accept: "application/json" } });
      renderPortfolioEvidence(evidence.sections);
    } catch (_) {
      renderPortfolioEvidence(null);
    }
  }

  async function portfolioAddTag() {
    const id = document.getElementById("portfolio-project").value;
    const name = document.getElementById("portfolio-tag").value.trim();
    clearPortfolioNotice();
    clearPortfolioError();
    clearErrorSummary("portfolio-error-summary");
    clearFieldError("portfolio-project", "portfolio-project-error");
    clearFieldError("portfolio-tag", "portfolio-tag-error");
    if (!id) {
      const message = "Select a managed project first.";
      showPortfolioError(message);
      setFieldError("portfolio-project", "portfolio-project-error", message);
      renderErrorSummary("portfolio-error-summary", "There is a problem adding the tag", [
        { fieldId: "portfolio-project", label: "Project", message },
      ]);
      return;
    }
    if (!name) {
      const message = "Enter a tag name.";
      showPortfolioError(message);
      setFieldError("portfolio-tag", "portfolio-tag-error", message);
      renderErrorSummary("portfolio-error-summary", "There is a problem adding the tag", [
        { fieldId: "portfolio-tag", label: "Tag name", message },
      ]);
      return;
    }
    try {
      await request(`/v1/admin/portfolio/${encodeURIComponent(id)}/tags`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ name }),
      });
      document.getElementById("portfolio-tag").value = "";
      showPortfolioNotice(`Tag “${name}” recorded for ${id}.`);
      await loadPortfolio();
    } catch (err) {
      const message = (err && err.message) || "The tag could not be recorded; nothing was changed.";
      showPortfolioError(message);
      setFieldError("portfolio-tag", "portfolio-tag-error", message);
      renderErrorSummary("portfolio-error-summary", "There is a problem adding the tag", [
        { fieldId: "portfolio-tag", label: "Tag name", message },
      ]);
    }
  }

  async function portfolioRecordReview() {
    const id = document.getElementById("portfolio-project").value;
    const confidence = document.getElementById("portfolio-confidence").value;
    clearPortfolioNotice();
    clearPortfolioError();
    clearErrorSummary("portfolio-error-summary");
    clearFieldError("portfolio-project", "portfolio-project-error");
    clearFieldError("portfolio-confidence", "portfolio-confidence-error");
    if (!id) {
      const message = "Select a managed project first.";
      showPortfolioError(message);
      setFieldError("portfolio-project", "portfolio-project-error", message);
      renderErrorSummary("portfolio-error-summary", "There is a problem recording the review", [
        { fieldId: "portfolio-project", label: "Project", message },
      ]);
      return;
    }
    if (!confidence) {
      const message = "Choose a review confidence.";
      showPortfolioError(message);
      setFieldError("portfolio-confidence", "portfolio-confidence-error", message);
      renderErrorSummary("portfolio-error-summary", "There is a problem recording the review", [
        { fieldId: "portfolio-confidence", label: "Review confidence", message },
      ]);
      return;
    }
    try {
      await request(`/v1/admin/portfolio/${encodeURIComponent(id)}/reviews`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ confidence }),
      });
      showPortfolioNotice(`Review recorded for ${id}.`);
      await loadPortfolio();
    } catch (err) {
      const message = (err && err.message) || "The review could not be recorded; nothing was changed.";
      showPortfolioError(message);
      setFieldError("portfolio-confidence", "portfolio-confidence-error", message);
      renderErrorSummary("portfolio-error-summary", "There is a problem recording the review", [
        { fieldId: "portfolio-confidence", label: "Review confidence", message },
      ]);
    }
  }

  function initPortfolio() {
    const confidence = document.getElementById("portfolio-confidence");
    PORTFOLIO_CONFIDENCE.forEach((value) => {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = value;
      confidence.append(option);
    });
    document.getElementById("portfolio-add-tag").addEventListener("click", portfolioAddTag);
    document.getElementById("portfolio-record-review").addEventListener("click", portfolioRecordReview);
    document.getElementById("portfolio-refresh").addEventListener("click", loadPortfolio);
    loadPortfolio();
  }

  // ---- Delivery controls (share pipeline: allowlist → preview → approve →
  // publish → reconcile → status)
  //
  // Boundary: the browser only ever sends a validated project `id` chosen
  // from the dropdown and a fixed set of share fields, plus the exact
  // preview digest it reviewed and an operation key. It never sends a
  // filesystem path, a command line or anything a shell would interpret.
  // Every mutating call requires an explicit confirmation tick and echoes
  // `plan_digest`; a changed or stale digest is refused with `effect: none`
  // and the refreshed preview is re-rendered. Publication and provider
  // outcomes are shown exactly as the journaled audit trail recorded them —
  // never a fabricated success. All values render as text via textContent.
  const DELIVERY_STATUS_LABELS = {
    published: "Published", failed: "Failed", unknown: "Unknown — reconcile",
    pending: "Pending", reconciled: "Reconciled", approved: "Approved",
    "not-run": "Not run", not_run: "Not run", disabled: "Disabled",
  };
  const delivery = { digest: null };

  function deliveryLabel(state) {
    return DELIVERY_STATUS_LABELS[state] || (state || "unknown");
  }

  function shortDigest(digest) {
    return digest ? `${String(digest).slice(0, 12)}…` : "—";
  }

  function showDeliveryNotice(message) {
    const notice = document.getElementById("delivery-notice");
    notice.textContent = message;
    notice.hidden = false;
  }

  function clearDeliveryNotice() {
    const notice = document.getElementById("delivery-notice");
    notice.hidden = true;
    notice.textContent = "";
  }

  function showDeliveryError(message) {
    const error = document.getElementById("delivery-error");
    error.textContent = message;
    error.hidden = false;
  }

  function clearDeliveryError() {
    const error = document.getElementById("delivery-error");
    error.hidden = true;
    error.textContent = "";
  }

  const DELIVERY_FIELDS = [
    ["delivery-project", "delivery-project-error"],
    ["delivery-title-input", "delivery-title-input-error"],
    ["delivery-summary", "delivery-summary-error"],
    ["delivery-category", "delivery-category-error"],
    ["delivery-source", "delivery-source-error"],
    ["delivery-opkey", "delivery-opkey-error"],
    ["delivery-reconcile-id", "delivery-reconcile-id-error"],
    ["delivery-reconcile-status", "delivery-reconcile-status-error"],
    ["delivery-reconcile-digest", "delivery-reconcile-digest-error"],
    ["delivery-lookup-key", "delivery-lookup-key-error"],
  ];

  function clearDeliveryFieldErrors() {
    for (const [fieldId, errorId] of DELIVERY_FIELDS) clearFieldError(fieldId, errorId);
  }

  // Report one failed delivery submission: the banner keeps the first
  // message, every failing field keeps its inline error, and focus moves
  // to the summary with a link per field.
  function deliverySubmitError(heading, failures) {
    if (failures.length) showDeliveryError(failures[0].message);
    clearErrorSummary("delivery-error-summary");
    for (const failure of failures) {
      if (failure.fieldId && failure.errorId) setFieldError(failure.fieldId, failure.errorId, failure.message);
    }
    renderErrorSummary("delivery-error-summary", heading, failures);
  }

  // A server refusal keeps its result rendering (see
  // `renderDeliveryActionResult`) and gains a focused summary so the
  // failure is announced with a target to land on.
  function deliveryRefused(action, body, status) {
    const message = (body && body.error && body.error.message) || `Delivery call returned HTTP ${status}.`;
    renderErrorSummary("delivery-error-summary", `${action} was refused`, [{ label: action, message }]);
  }

  function renderDeliveryActionResult(result, status) {
    const box = document.getElementById("delivery-action-result");
    box.textContent = "";
    box.hidden = false;
    const head = document.createElement("p");
    head.className = "wb-plan-head";
    const note = result.note || (result.error && result.error.message)
      || `Delivery call returned HTTP ${status}.`;
    head.textContent = note;
    box.append(head);
    const effect = document.createElement("p");
    effect.className = "muted";
    effect.textContent = `Effect: ${result.effect || (result.accepted ? "external-delivery-dispatched" : "none")}.`;
    box.append(effect);
    if (result.publication) {
      const pub = result.publication;
      const detail = document.createElement("p");
      detail.className = "muted";
      detail.textContent = `Operation ${pub.operation_key || "—"} · ${deliveryLabel(pub.status)} · digest ${shortDigest(pub.manifest_sha256)} · publisher ${pub.publisher || "—"}.`;
      box.append(detail);
    }
    if (result.preview && result.preview.manifest_sha256) {
      // A stale/changed digest returns the refreshed preview; adopt it so the
      // operator re-confirms against real state rather than acting blind.
      delivery.digest = result.preview.manifest_sha256;
      const refreshed = document.createElement("p");
      refreshed.className = "muted";
      refreshed.textContent = "A fresh preview is ready below — review it and confirm again to proceed.";
      box.append(refreshed);
    }
    if (result.approval) {
      const detail = document.createElement("p");
      detail.className = "muted";
      detail.textContent = `Approval revision ${result.approval.revision} bound to ${shortDigest(result.approval.manifest_sha256)}.`;
      box.append(detail);
    }
  }

  function renderDeliveryPreview(preview) {
    const list = document.getElementById("delivery-preview");
    list.replaceChildren();
    if (!preview || !preview.manifest_sha256) {
      list.append(detailRow("Preview", "Unavailable — the registry could not be read."));
      delivery.digest = null;
      return;
    }
    delivery.digest = preview.manifest_sha256;
    list.append(
      detailRow("Manifest revision", preview.manifest_revision),
      detailRow("Preview reference", shortDigest(preview.manifest_sha256)),
      detailRow("Candidate projects", preview.project_count),
      detailRow("Approvable", preview.approvable ? "Yes" : "No"),
    );
    const findings = preview.findings || [];
    if (findings.length) {
      const ul = document.createElement("ul");
      ul.className = "wb-findings";
      for (const finding of findings) {
        const li = document.createElement("li");
        li.textContent = `${finding.project_id} — ${finding.field}: ${finding.code}`;
        ul.append(li);
      }
      list.append(li_wrap(ul));
    } else {
      list.append(detailRow("Findings", "None — the manifest is clean."));
    }
  }

  // Wrap a node so it can sit inside the detail list without breaking layout.
  function li_wrap(node) {
    const row = document.createElement("div");
    row.className = "detail-row";
    const dt = document.createElement("dt");
    dt.textContent = "Blocking findings";
    const dd = document.createElement("dd");
    dd.append(node);
    row.append(dt, dd);
    return row;
  }

  function renderDeliveryTarget(target) {
    const el = document.getElementById("delivery-target");
    if (!target) { el.textContent = ""; return; }
    el.textContent = target.configured
      ? `Publication target: configured on this host (${target.publisher || "local export"}). The browser never names a path.`
      : "Publication target: NOT configured (server-side). A browser publish is refused as a typed prerequisite until an operator configures it in a terminal.";
  }

  function renderDeliveryProvider(provider) {
    const el = document.getElementById("delivery-provider");
    const rows = (provider && provider.rows) || [];
    if (!rows.length) { el.textContent = "Provider matrix unavailable."; return; }
    const notRun = rows.filter((row) => (row.status || "").includes("not-run") || (row.status || "").includes("not_run")).length;
    el.textContent = `Providers: ${rows.length} listed, ${notRun} not-run. No live probe runs on page load; delivery uses only Forge's local export.`;
  }

  function renderDeliveryAllowlist(records) {
    const body = document.getElementById("delivery-allowlist-rows");
    body.textContent = "";
    document.getElementById("delivery-allowlist-empty").hidden = records.length > 0;
    records.forEach((record) => {
      const row = document.createElement("tr");
      row.append(
        textCell(record.project_id),
        textCell(record.title),
        textCell(record.visibility),
        textCell(record.state),
        textCell(record.revision == null ? "—" : String(record.revision)),
      );
      body.append(row);
    });
  }

  function renderDeliveryApproval(approval) {
    const list = document.getElementById("delivery-approval");
    list.replaceChildren();
    if (!approval) {
      list.append(detailRow("Latest approval", "None yet — approve a preview digest first."));
      return;
    }
    list.append(
      detailRow("Approval revision", approval.revision),
      detailRow("Approved reference", shortDigest(approval.manifest_sha256)),
      detailRow("Project count", approval.project_count),
      detailRow("State", deliveryLabel(approval.state)),
      detailRow("Actor", approval.actor),
    );
  }

  function renderDeliveryPublications(publications) {
    const body = document.getElementById("delivery-publication-rows");
    body.textContent = "";
    document.getElementById("delivery-publication-empty").hidden = publications.length > 0;
    publications.forEach((attempt) => {
      const row = document.createElement("tr");
      row.append(
        textCell(attempt.operation_key),
        textCell(deliveryLabel(attempt.status)),
        textCell(shortDigest(attempt.manifest_sha256)),
        textCell(attempt.actor),
        textCell(attempt.finished_at || "—"),
      );
      body.append(row);
    });
  }

  function renderDeliveryUnreconciled(attempt) {
    const banner = document.getElementById("delivery-unreconciled");
    if (!attempt) { banner.hidden = true; banner.textContent = ""; return; }
    banner.textContent = `Unreconciled publication: operation ${attempt.operation_key || "—"} (id ${attempt.publication_id ?? "—"}, digest ${shortDigest(attempt.manifest_sha256)}). Forge cannot vouch for this dispatch — reconcile it before publishing a new revision.`;
    banner.hidden = false;
    // Pre-fill the reconcile reference from status data so the operator
    // reviews it instead of hand-copying a hash.
    if (attempt.manifest_sha256) {
      document.getElementById("delivery-reconcile-digest").value = attempt.manifest_sha256;
    }
    if (attempt.publication_id !== undefined && attempt.publication_id !== null) {
      document.getElementById("delivery-reconcile-id").value = String(attempt.publication_id);
    }
  }

  function populateDeliveryProjects(projects) {
    const select = document.getElementById("delivery-project");
    select.textContent = "";
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Choose a project…";
    select.append(placeholder);
    managedProjects(projects).forEach((project) => {
      const option = document.createElement("option");
      option.value = project.identity;
      option.textContent = project.name || project.identity;
      select.append(option);
    });
  }

  async function loadDelivery() {
    clearDeliveryError();
    try {
      const data = await request("/v1/admin/delivery", { headers: { Accept: "application/json" } });
      renderDeliveryPreview(data.preview);
      renderDeliveryTarget(data.target);
      renderDeliveryProvider(data.provider);
      renderDeliveryAllowlist(data.allowlist || []);
      renderDeliveryApproval(data.approval);
      renderDeliveryPublications(data.publications || []);
      renderDeliveryUnreconciled(data.unreconciled);
      populateDeliveryProjects((window.__forgeProjects || []));
    } catch (err) {
      showDeliveryError(err.message || "Delivery controls unavailable. Start the Forge API and reload.");
    }
  }

  // Null when no preview digest is loaded; callers report the failure
  // through their own error summary so the banner text stays in one place.
  function requireDeliveryDigest() {
    if (!delivery.digest) {
      return null;
    }
    return delivery.digest;
  }

  async function deliverySetAllowlist(isRemove) {
    clearDeliveryNotice();
    clearDeliveryError();
    clearErrorSummary("delivery-error-summary");
    clearDeliveryFieldErrors();
    const id = document.getElementById("delivery-project").value;
    if (!id) { deliverySubmitError("There is a problem with the share record", [{ fieldId: "delivery-project", errorId: "delivery-project-error", label: "Project", message: "Choose a project first." }]); return; }
    const digest = requireDeliveryDigest();
    if (!digest) { deliverySubmitError("There is a problem with the share record", [{ label: "Preview", message: "No preview digest is loaded yet. Refresh delivery first." }]); return; }
    const confirmEl = document.getElementById("delivery-set-confirm");
    if (!confirmEl.checked) { deliverySubmitError("There is a problem with the share record", [{ fieldId: "delivery-set-confirm", label: "Confirmation", message: "Tick the confirmation box — this changes what may become public." }]); return; }
    const path = isRemove
      ? `/v1/admin/delivery/allowlist/${encodeURIComponent(id)}/remove`
      : `/v1/admin/delivery/allowlist/${encodeURIComponent(id)}`;
    const payload = isRemove
      ? { confirm: true, plan_digest: digest }
      : {
        confirm: true,
        plan_digest: digest,
        title: document.getElementById("delivery-title-input").value.trim(),
        summary: document.getElementById("delivery-summary").value.trim(),
        category: document.getElementById("delivery-category").value.trim(),
        source_url: document.getElementById("delivery-source").value.trim(),
      };
    document.getElementById("delivery-set").disabled = true;
    document.getElementById("delivery-remove").disabled = true;
    const { status, ok, body } = await requestStatus(path, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify(payload),
    });
    document.getElementById("delivery-set").disabled = false;
    document.getElementById("delivery-remove").disabled = false;
    renderDeliveryActionResult(body, status);
    if (!ok) deliveryRefused("Allowlist update", body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryApprove() {
    clearDeliveryNotice();
    clearDeliveryError();
    clearErrorSummary("delivery-error-summary");
    clearDeliveryFieldErrors();
    const digest = requireDeliveryDigest();
    if (!digest) { deliverySubmitError("There is a problem approving the preview", [{ label: "Preview", message: "No preview digest is loaded yet. Refresh delivery first." }]); return; }
    const confirmEl = document.getElementById("delivery-approve-confirm");
    if (!confirmEl.checked) { deliverySubmitError("There is a problem approving the preview", [{ fieldId: "delivery-approve-confirm", label: "Confirmation", message: "Tick the confirmation box — this approves the manifest for publication." }]); return; }
    const { status, ok, body } = await requestStatus("/v1/admin/delivery/approve", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ confirm: true, plan_digest: digest }),
    });
    renderDeliveryActionResult(body, status);
    if (!ok) deliveryRefused("Approval", body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryPublish() {
    clearDeliveryNotice();
    clearDeliveryError();
    clearErrorSummary("delivery-error-summary");
    clearDeliveryFieldErrors();
    const digest = requireDeliveryDigest();
    if (!digest) { deliverySubmitError("There is a problem publishing", [{ label: "Preview", message: "No preview digest is loaded yet. Refresh delivery first." }]); return; }
    const key = document.getElementById("delivery-opkey").value.trim();
    if (!key) { deliverySubmitError("There is a problem publishing", [{ fieldId: "delivery-opkey", errorId: "delivery-opkey-error", label: "Operation key", message: "Enter an operation key (the idempotency identity for this publish)." }]); return; }
    const confirmEl = document.getElementById("delivery-publish-confirm");
    if (!confirmEl.checked) { deliverySubmitError("There is a problem publishing", [{ fieldId: "delivery-publish-confirm", label: "Confirmation", message: "Tick the confirmation box — this writes the approved manifest to the server target." }]); return; }
    document.getElementById("delivery-publish").disabled = true;
    const { status, ok, body } = await requestStatus("/v1/admin/delivery/publish", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ confirm: true, plan_digest: digest, operation_key: key }),
    });
    document.getElementById("delivery-publish").disabled = false;
    renderDeliveryActionResult(body, status);
    if (!ok) deliveryRefused("Publish", body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryReconcile() {
    clearDeliveryNotice();
    clearDeliveryError();
    clearErrorSummary("delivery-error-summary");
    clearDeliveryFieldErrors();
    const failures = [];
    const idRaw = document.getElementById("delivery-reconcile-id").value.trim();
    const publicationId = Number(idRaw);
    if (!idRaw || !Number.isInteger(publicationId)) failures.push({ fieldId: "delivery-reconcile-id", errorId: "delivery-reconcile-id-error", label: "Publication id", message: "Enter the numeric publication id of the ambiguous attempt." });
    const statusChoice = document.getElementById("delivery-reconcile-status").value;
    if (!statusChoice) failures.push({ fieldId: "delivery-reconcile-status", errorId: "delivery-reconcile-status-error", label: "Observed outcome", message: "Choose the outcome you observed (published or failed)." });
    const digest = document.getElementById("delivery-reconcile-digest").value.trim();
    if (!digest) failures.push({ fieldId: "delivery-reconcile-digest", errorId: "delivery-reconcile-digest-error", label: "Attempt reference", message: "The attempt reference is missing — refresh delivery and try again." });
    if (failures.length) { deliverySubmitError("There is a problem reconciling the attempt", failures); return; }
    const confirmEl = document.getElementById("delivery-reconcile-confirm");
    if (!confirmEl.checked) { deliverySubmitError("There is a problem reconciling the attempt", [{ fieldId: "delivery-reconcile-confirm", label: "Confirmation", message: "Tick the confirmation box — this records the outcome as your explicit statement." }]); return; }
    const { status, ok, body } = await requestStatus("/v1/admin/delivery/reconcile", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ confirm: true, publication_id: publicationId, status: statusChoice, plan_digest: digest }),
    });
    renderDeliveryActionResult(body, status);
    if (!ok) deliveryRefused("Reconcile", body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryLookup() {
    clearDeliveryNotice();
    clearDeliveryError();
    clearErrorSummary("delivery-error-summary");
    clearDeliveryFieldErrors();
    const key = document.getElementById("delivery-lookup-key").value.trim();
    if (!key) { deliverySubmitError("There is a problem looking up the operation", [{ fieldId: "delivery-lookup-key", errorId: "delivery-lookup-key-error", label: "Operation key", message: "Enter an operation key to look up." }]); return; }
    let result;
    try {
      result = await request(`/v1/admin/delivery/operation/${encodeURIComponent(key)}`, { headers: { Accept: "application/json" } });
    } catch (err) {
      deliverySubmitError("There is a problem looking up the operation", [{ fieldId: "delivery-lookup-key", errorId: "delivery-lookup-key-error", label: "Operation key", message: (err && err.message) || "No publication with that operation key is recorded here." }]);
      return;
    }
    const box = document.getElementById("delivery-action-result");
    box.textContent = "";
    box.hidden = false;
    const head = document.createElement("p");
    head.className = "wb-plan-head";
    head.textContent = `Operation ${key}: ${deliveryLabel((result.publication || {}).status)}.`;
    box.append(head);
    const guidance = document.createElement("p");
    guidance.className = "muted";
    guidance.textContent = result.guidance || "The attempt is recorded as shown.";
    box.append(guidance);
  }

  function initDelivery(projects) {
    window.__forgeProjects = projects;
    document.getElementById("delivery-refresh").addEventListener("click", loadDelivery);
    document.getElementById("delivery-set").addEventListener("click", () => deliverySetAllowlist(false));
    document.getElementById("delivery-remove").addEventListener("click", () => deliverySetAllowlist(true));
    document.getElementById("delivery-approve").addEventListener("click", deliveryApprove);
    document.getElementById("delivery-publish").addEventListener("click", deliveryPublish);
    document.getElementById("delivery-reconcile").addEventListener("click", deliveryReconcile);
    document.getElementById("delivery-lookup").addEventListener("click", deliveryLookup);
    loadDelivery();
  }

  // The fleet readiness tile: counts of every registered project by its
  // read-only overall status. Text-only; a failed read shows em dashes, never
  // a stale or invented count.
  async function loadFleetReadiness() {
    const ids = ["fleet-healthy", "fleet-issues", "fleet-stale", "fleet-unavailable", "fleet-total"];
    const set = (id, value) => {
      const el = document.getElementById(id);
      if (el) el.textContent = (value === undefined || value === null) ? "—" : String(value);
    };
    let data;
    try {
      data = await request("/v1/admin/status", { headers: { Accept: "application/json" } });
    } catch (_) {
      for (const id of ids) set(id, "—");
      return;
    }
    const counts = data.counts || {};
    set("fleet-healthy", counts.healthy ?? 0);
    set("fleet-issues", counts.issues ?? 0);
    set("fleet-stale", counts.stale ?? 0);
    set("fleet-unavailable", counts.unavailable ?? 0);
    set("fleet-total", data.total ?? 0);
  }

  async function dashboardPage() {
    let state;
    try { state = await session(); }
    catch (_) { showDashboardError("Forge API is unavailable. Start the API and reload this page."); return; }
    if (!state.authenticated) {
      // Carry the deep link through sign-in: after authenticating, the
      // login page returns here (see `loginNextTarget`). A bare entry
      // still lands on `index.html` because an invalid `next` is dropped.
      const here = window.location.pathname + window.location.search;
      window.location.replace(`login.html?next=${encodeURIComponent(here)}`);
      return;
    }
    loadCommands();
    let projects = [];
    try {
      const data = await request("/v1/admin/projects", { headers: { Accept: "application/json" } });
      projects = data.projects || [];
      window.__forgeProjects = projects;
      renderSources(data.sources || []);
      document.getElementById("summary-total").textContent = String(data.summary?.registered ?? 0);
      document.getElementById("summary-profiles").textContent = String(data.summary?.profiles ?? new Set(projects.map((project) => project.profile)).size);
      document.getElementById("summary-evidence").textContent = String(data.summary?.with_evidence ?? 0);
      loadFleetReadiness();
      document.getElementById("updated-label").textContent = `Updated ${new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
      const hasOthers = projects.some((project) => !project.is_self);
      if (!hasOthers && data.summary && data.summary.registered === 0) {
        document.getElementById("empty-state").hidden = false;
      }
      // Restore the operator's typed search/filter context from before a
      // reload (see `restoreFilterState`), then render through the inputs
      // so the table matches them from the first paint.
      const restoredFilters = restoreFilterState();
      if (projects.length) {
        if (restoredFilters) renderProjectsFromInputs();
        else renderProjects(projects);
        document.querySelector(".table-scroll").hidden = false;
      }
      initWorkbench(projects);
      initPortfolio();
      initDelivery(projects);
      initWorkspaceOnboarding();
      wsDiscover();
      const refresh = () => renderProjects(window.__forgeProjects || projects);
      document.getElementById("project-search").addEventListener("input", refresh);
      document.getElementById("source-filter").addEventListener("change", refresh);
      initFleetFilters();
      initFilterStatePersistence();
    } catch (_) { showDashboardError("Forge could not load project data. Reload to try again."); }
    document.getElementById("sign-out").addEventListener("click", async (event) => {
      const button = event.currentTarget; button.disabled = true;
      try { await request("/v1/admin/session", { method: "DELETE", headers: { Accept: "application/json" } }); }
      catch (_) { showDashboardError("Sign out failed. Check the API connection and try again."); button.disabled = false; return; }
      window.location.replace("login.html");
    });
  }

  function showDashboardError(message) {
    const error = document.getElementById("dashboard-error");
    error.textContent = message;
    error.hidden = false;
    document.getElementById("project-count").textContent = "Project data unavailable";
  }

  if (page === "login") loginPage();
  if (page === "dashboard") {
    initRouter();
    renderRoute();
    dashboardPage();
  }
})();
