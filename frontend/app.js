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

  async function loginPage() {
    const form = document.getElementById("login-form");
    const error = document.getElementById("login-error");
    const setup = document.getElementById("setup-state");
    try {
      const state = await session();
      if (state.authenticated) { window.location.replace("index.html"); return; }
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
        window.location.assign("index.html");
      } catch (_) {
        error.textContent = "Email or password is incorrect.";
        error.hidden = false;
        form.elements.password.value = "";
        form.elements.password.focus();
      } finally {
        button.disabled = false;
        button.innerHTML = 'Sign in to your workspace <span aria-hidden="true">→</span>';
      }
    });
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

  const SOURCE_LABELS = { self: "Forge (self)", registry: "Registered", inventory: "Inventory", fleet: "Workspace" };
  const SOURCE_BADGE = { self: "self", registry: "registry", inventory: "inventory", fleet: "workspace" };
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

  function renderProjects(projects) {
    const body = document.getElementById("project-rows");
    const query = document.getElementById("project-search").value.trim().toLowerCase();
    const sourceFilter = document.getElementById("source-filter").value;
    const filtered = projects.filter((project) => {
      if (sourceFilter && project.source !== sourceFilter) return false;
      return `${project.name} ${project.identity} ${project.profile}`.toLowerCase().includes(query);
    });
    body.replaceChildren();
    for (const project of filtered) {
      const row = document.createElement("tr");
      if (project.is_self) row.className = "row-self";
      if (project.conflict) row.className = "row-conflict";

      const identity = document.createElement("td");
      const name = document.createElement("div"); name.className = "project-name";
      const avatar = document.createElement("span"); avatar.className = "project-avatar"; avatar.setAttribute("aria-hidden", "true"); avatar.textContent = (project.name || "?").slice(0, 1).toUpperCase();
      const label = document.createElement("span"); label.textContent = project.name;
      name.append(avatar, label);
      if (project.is_self) name.append(makeBadge("This is Forge", "self"));
      if (project.conflict) name.append(makeBadge("Identity conflict", "conflict"));
      identity.append(name); row.append(identity);

      const source = document.createElement("td");
      const sourceWrap = document.createElement("div"); sourceWrap.className = "source-cell";
      sourceWrap.append(makeBadge(SOURCE_LABELS[project.source] || project.source, SOURCE_BADGE[project.source] || "observed"));
      if (project.freshness && project.freshness !== "fresh") sourceWrap.append(makeBadge(project.freshness, project.freshness));
      source.append(sourceWrap); row.append(source);

      row.append(textCell(project.profile));

      const state = document.createElement("td");
      const stateLabel = document.createElement("span"); stateLabel.className = "state-label";
      const dot = document.createElement("span"); dot.className = `state-dot ${project.state === "done" ? "state-observed" : ""} ${project.freshness === "stale" ? "state-stale" : ""}`; dot.setAttribute("aria-hidden", "true");
      stateLabel.append(dot, document.createTextNode(project.state || "No activity")); state.append(stateLabel); row.append(state);

      const evidence = document.createElement("td"); const chips = document.createElement("div"); chips.className = "evidence-list";
      if (project.evidence?.length) {
        for (const item of project.evidence) { const chip = document.createElement("span"); chip.className = `evidence-chip evidence-${item.status}`; chip.textContent = `${item.source}: ${item.status}`; chips.append(chip); }
      } else { const chip = document.createElement("span"); chip.className = "evidence-chip"; chip.textContent = "No evidence"; chips.append(chip); }
      evidence.append(chips); row.append(evidence);

      row.append(textCell(project.lifecycle || "Unclassified"));

      const access = document.createElement("td");
      const accessWrap = document.createElement("div"); accessWrap.className = "access-cell";
      accessWrap.append(makeBadge(project.management === "managed" ? "Managed" : project.management === "self" ? "Self" : "Observed", `mgmt-${project.management}`));
      // Capabilities are shown honestly as labels only: this change exposes no
      // project mutation endpoint, so no operation is rendered as an action.
      for (const capability of project.capabilities || []) accessWrap.append(makeBadge(capability, "capability"));
      if (!(project.capabilities && project.capabilities.length)) accessWrap.append(makeBadge("Read-only", "readonly"));
      access.append(accessWrap); row.append(access);

      body.append(row);
    }
    document.getElementById("no-results").hidden = projects.length === 0 || filtered.length > 0;
    document.querySelector(".table-scroll").hidden = filtered.length === 0;
    document.getElementById("project-count").textContent = `${filtered.length} of ${projects.length} project${projects.length === 1 ? "" : "s"}`;
  }

  function renderCommands(commands) {
    const body = document.getElementById("command-rows");
    const query = document.getElementById("command-search").value.trim().toLowerCase();
    const categoryFilter = document.getElementById("category-filter").value;
    const availabilityFilter = document.getElementById("availability-filter").value;
    const filtered = commands.filter((command) => {
      if (categoryFilter && command.category !== categoryFilter) return false;
      if (availabilityFilter && command.availability !== availabilityFilter) return false;
      return `${command.id} ${command.summary} ${command.cli_invocation}`.toLowerCase().includes(query);
    });
    body.replaceChildren();
    for (const command of filtered) {
      const row = document.createElement("tr");

      const name = document.createElement("td");
      const idWrap = document.createElement("div");
      idWrap.className = `command-id depth-${(command.id.match(/\./g) || []).length}`;
      const idLabel = document.createElement("span");
      idLabel.textContent = command.label;
      const pathNote = document.createElement("span");
      pathNote.className = "command-path";
      pathNote.textContent = command.parent_id ? `${command.parent_id} › ` : "";
      idWrap.append(pathNote, idLabel);
      name.append(idWrap);
      row.append(name);

      const category = document.createElement("td");
      category.append(makeBadge(CATEGORY_LABELS[command.category] || command.category, "category"));
      row.append(category);

      const risk = document.createElement("td");
      risk.append(makeBadge(RISK_LABELS[command.risk] || command.risk, RISK_BADGE[command.risk] || "risk-read"));
      row.append(risk);

      const state = document.createElement("td");
      state.append(makeBadge(AVAILABILITY_LABELS[command.availability] || command.availability, AVAILABILITY_BADGE[command.availability] || "cat-gap"));
      row.append(state);

      // The CLI invocation is display-only text inside <code>: the browser
      // never executes it and the API has no shell/eval route to run it.
      const invocation = document.createElement("td");
      const code = document.createElement("code");
      code.textContent = command.cli_invocation;
      invocation.append(code);
      row.append(invocation);

      const guidance = document.createElement("td");
      const guidanceText = document.createElement("p");
      guidanceText.className = "command-guidance";
      guidanceText.textContent = command.availability === "web"
        ? `Available through this dashboard (${command.route}).`
        : command.reason || "—";
      guidance.append(guidanceText);
      row.append(guidance);

      body.append(row);
    }
    document.getElementById("command-no-results").hidden = commands.length === 0 || filtered.length > 0;
    document.getElementById("commands-table").hidden = filtered.length === 0;
    document.getElementById("command-count").textContent = `${filtered.length} of ${commands.length} CLI command${commands.length === 1 ? "" : "s"}`;
  }

  function showCommandsError(message) {    const error = document.getElementById("commands-error");
    error.textContent = message;
    error.hidden = false;
    document.getElementById("commands-table").hidden = true;
    document.getElementById("command-no-results").hidden = true;
    document.getElementById("command-count").textContent = "Command catalog unavailable";
  }

  function loadCommands() {
    // The catalog is static descriptive metadata. A failure here never
    // hides the project fleet, and it shows an honest unavailable state.
    return request("/v1/admin/commands", { headers: { Accept: "application/json" } })
      .then((data) => {
        const commands = data.commands || [];
        const select = document.getElementById("category-filter");
        for (const category of data.categories || []) {
          const option = document.createElement("option");
          option.value = category.id;
          option.textContent = `${category.label} (${category.count})`;
          select.append(option);
        }
        const refresh = () => renderCommands(commands);
        document.getElementById("command-search").addEventListener("input", refresh);
        document.getElementById("category-filter").addEventListener("change", refresh);
        document.getElementById("availability-filter").addEventListener("change", refresh);
        refresh();
      })
      .catch(() => showCommandsError("Command catalog unavailable. Start the Forge API and reload; every CLI command stays discoverable in the terminal meanwhile."));
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
  const workbench = { id: null, digest: null, idempotencyKey: null };

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

  function renderWorkflows(workflows) {
    const list = document.getElementById("wb-workflows");
    list.replaceChildren();
    for (const workflow of workflows) {
      const li = document.createElement("li"); li.className = "workflow-item";
      const name = document.createElement("strong"); name.textContent = workflow.label || workflow.id;
      li.append(name, " ", makeBadge(AVAILABILITY_LABELS[workflow.availability] || workflow.availability, AVAILABILITY_BADGE[workflow.availability] || "cat-gap"));
      if (!workflow.available) {
        const why = document.createElement("p"); why.className = "workflow-reason"; why.textContent = workflow.reason || "Not available in the browser.";
        li.append(why);
      } else {
        const where = document.createElement("p"); where.className = "workflow-reason"; where.textContent = `Runs through ${workflow.route || "a typed in-process operation"}.`;
        li.append(where);
      }
      list.append(li);
    }
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
    renderWorkflows(data.workflows || []);
    renderOperations(data.operations || []);
    populateFeatures(data.manifest || {});
    document.getElementById("workbench-title").scrollIntoView({ behavior: "smooth", block: "start" });
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
    const digest = document.createElement("p"); digest.className = "wb-digest";
    digest.textContent = `Plan digest: ${result.plan_digest}`;
    box.append(digest);
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
      note.textContent = "A refreshed plan and digest are shown below. Review it and confirm again to apply.";
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
    head.textContent = `Accepted as journaled operation ${result.operation_id}. The apply ran through Forge's in-process upgrade, recorded in the journal.`;
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
      loadWorkbenchDetail(id);
    });
    select.addEventListener("change", () => {
      if (select.value) loadWorkbenchDetail(select.value);
      else { document.getElementById("workbench-body").hidden = true; document.getElementById("workbench-empty").hidden = false; clearWorkbenchNotice(); }
    });
    document.getElementById("wb-plan").addEventListener("click", planUpgrade);
    document.getElementById("wb-apply").addEventListener("click", applyUpgrade);
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
    if (!id) { showPortfolioError("Select a managed project first."); return; }
    if (!name) { showPortfolioError("Enter a tag name."); return; }
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
      showPortfolioError(err.message || "The tag could not be recorded; nothing was changed.");
    }
  }

  async function portfolioRecordReview() {
    const id = document.getElementById("portfolio-project").value;
    const confidence = document.getElementById("portfolio-confidence").value;
    clearPortfolioNotice();
    clearPortfolioError();
    if (!id) { showPortfolioError("Select a managed project first."); return; }
    if (!confidence) { showPortfolioError("Choose a review confidence."); return; }
    try {
      await request(`/v1/admin/portfolio/${encodeURIComponent(id)}/reviews`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ confidence }),
      });
      showPortfolioNotice(`Review recorded for ${id}.`);
      await loadPortfolio();
    } catch (err) {
      showPortfolioError(err.message || "The review could not be recorded; nothing was changed.");
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
      refreshed.textContent = `Refreshed preview digest: ${delivery.digest}. Review it and confirm again to proceed.`;
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
      detailRow("Preview digest", preview.manifest_sha256),
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
      detailRow("Approved digest", approval.manifest_sha256),
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

  function requireDeliveryDigest() {
    if (!delivery.digest) {
      showDeliveryError("No preview digest is loaded yet. Refresh delivery first.");
      return null;
    }
    return delivery.digest;
  }

  async function deliverySetAllowlist(isRemove) {
    clearDeliveryNotice();
    clearDeliveryError();
    const id = document.getElementById("delivery-project").value;
    if (!id) { showDeliveryError("Choose a project first."); return; }
    const digest = requireDeliveryDigest();
    if (!digest) return;
    const confirmEl = document.getElementById("delivery-set-confirm");
    if (!confirmEl.checked) { showDeliveryError("Tick the confirmation box — this changes what may become public."); return; }
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
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryApprove() {
    clearDeliveryNotice();
    clearDeliveryError();
    const digest = requireDeliveryDigest();
    if (!digest) return;
    const confirmEl = document.getElementById("delivery-approve-confirm");
    if (!confirmEl.checked) { showDeliveryError("Tick the confirmation box — this approves the manifest for publication."); return; }
    const { status, ok, body } = await requestStatus("/v1/admin/delivery/approve", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ confirm: true, plan_digest: digest }),
    });
    renderDeliveryActionResult(body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryPublish() {
    clearDeliveryNotice();
    clearDeliveryError();
    const digest = requireDeliveryDigest();
    if (!digest) return;
    const key = document.getElementById("delivery-opkey").value.trim();
    if (!key) { showDeliveryError("Enter an operation key (the idempotency identity for this publish)."); return; }
    const confirmEl = document.getElementById("delivery-publish-confirm");
    if (!confirmEl.checked) { showDeliveryError("Tick the confirmation box — this writes the approved manifest to the server target."); return; }
    document.getElementById("delivery-publish").disabled = true;
    const { status, ok, body } = await requestStatus("/v1/admin/delivery/publish", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ confirm: true, plan_digest: digest, operation_key: key }),
    });
    document.getElementById("delivery-publish").disabled = false;
    renderDeliveryActionResult(body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryReconcile() {
    clearDeliveryNotice();
    clearDeliveryError();
    const idRaw = document.getElementById("delivery-reconcile-id").value.trim();
    const publicationId = Number(idRaw);
    if (!idRaw || !Number.isInteger(publicationId)) { showDeliveryError("Enter the numeric publication id of the ambiguous attempt."); return; }
    const statusChoice = document.getElementById("delivery-reconcile-status").value;
    if (!statusChoice) { showDeliveryError("Choose the outcome you observed (published or failed)."); return; }
    const digest = document.getElementById("delivery-reconcile-digest").value.trim();
    if (!digest) { showDeliveryError("Enter the attempt's recorded digest — reconciliation binds to the exact ambiguous publication."); return; }
    const confirmEl = document.getElementById("delivery-reconcile-confirm");
    if (!confirmEl.checked) { showDeliveryError("Tick the confirmation box — this records the outcome as your explicit statement."); return; }
    const { status, ok, body } = await requestStatus("/v1/admin/delivery/reconcile", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ confirm: true, publication_id: publicationId, status: statusChoice, plan_digest: digest }),
    });
    renderDeliveryActionResult(body, status);
    confirmEl.checked = false;
    await loadDelivery();
  }

  async function deliveryLookup() {
    clearDeliveryNotice();
    clearDeliveryError();
    const key = document.getElementById("delivery-lookup-key").value.trim();
    if (!key) { showDeliveryError("Enter an operation key to look up."); return; }
    let result;
    try {
      result = await request(`/v1/admin/delivery/operation/${encodeURIComponent(key)}`, { headers: { Accept: "application/json" } });
    } catch (err) {
      showDeliveryError(err.message || "No publication with that operation key is recorded here.");
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

  async function dashboardPage() {
    let state;
    try { state = await session(); }
    catch (_) { showDashboardError("Forge API is unavailable. Start the API and reload this page."); return; }
    if (!state.authenticated) { window.location.replace("login.html"); return; }
    loadCommands();
    let projects = [];
    try {
      const data = await request("/v1/admin/projects", { headers: { Accept: "application/json" } });
      projects = data.projects || [];
      renderSources(data.sources || []);
      document.getElementById("summary-total").textContent = String(data.summary?.registered ?? 0);
      document.getElementById("summary-profiles").textContent = String(data.summary?.profiles ?? new Set(projects.map((project) => project.profile)).size);
      document.getElementById("summary-evidence").textContent = String(data.summary?.with_evidence ?? 0);
      document.getElementById("updated-label").textContent = `Updated ${new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
      const hasOthers = projects.some((project) => !project.is_self);
      if (!hasOthers && data.summary && data.summary.registered === 0) {
        document.getElementById("empty-state").hidden = false;
      }
      if (projects.length) {
        renderProjects(projects);
        document.querySelector(".table-scroll").hidden = false;
      }
      initWorkbench(projects);
      initPortfolio();
      initDelivery(projects);
      const refresh = () => renderProjects(projects);
      document.getElementById("project-search").addEventListener("input", refresh);
      document.getElementById("source-filter").addEventListener("change", refresh);
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
  if (page === "dashboard") dashboardPage();
})();
