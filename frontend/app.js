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

  function renderProjects(projects) {
    const body = document.getElementById("project-rows");
    const query = document.getElementById("project-search").value.trim().toLowerCase();
    const filtered = projects.filter((project) => `${project.id} ${project.profile}`.toLowerCase().includes(query));
    body.replaceChildren();
    for (const project of filtered) {
      const row = document.createElement("tr");
      const identity = document.createElement("td");
      const name = document.createElement("div"); name.className = "project-name";
      const avatar = document.createElement("span"); avatar.className = "project-avatar"; avatar.setAttribute("aria-hidden", "true"); avatar.textContent = (project.id || "?").slice(0, 1).toUpperCase();
      const label = document.createElement("span"); label.textContent = project.id;
      name.append(avatar, label); identity.append(name); row.append(identity);
      row.append(textCell(project.profile));
      const state = document.createElement("td");
      const stateLabel = document.createElement("span"); stateLabel.className = "state-label";
      const dot = document.createElement("span"); dot.className = `state-dot ${project.state === "done" ? "state-observed" : ""}`; dot.setAttribute("aria-hidden", "true");
      stateLabel.append(dot, document.createTextNode(project.state || "No activity")); state.append(stateLabel); row.append(state);
      const evidence = document.createElement("td"); const chips = document.createElement("div"); chips.className = "evidence-list";
      if (project.evidence?.length) {
        for (const item of project.evidence) { const chip = document.createElement("span"); chip.className = `evidence-chip evidence-${item.status}`; chip.textContent = `${item.source}: ${item.status}`; chips.append(chip); }
      } else { const chip = document.createElement("span"); chip.className = "evidence-chip"; chip.textContent = "No evidence"; chips.append(chip); }
      evidence.append(chips); row.append(evidence);
      row.append(textCell(project.lifecycle || "Unclassified"));
      body.append(row);
    }
    document.getElementById("no-results").hidden = projects.length === 0 || filtered.length > 0;
    document.querySelector(".table-scroll").hidden = filtered.length === 0;
    document.getElementById("project-count").textContent = `${filtered.length} of ${projects.length} project${projects.length === 1 ? "" : "s"}`;
  }

  async function dashboardPage() {
    let state;
    try { state = await session(); }
    catch (_) { showDashboardError("Forge API is unavailable. Start the API and reload this page."); return; }
    if (!state.authenticated) { window.location.replace("login.html"); return; }
    let projects;
    try {
      const data = await request("/v1/admin/projects", { headers: { Accept: "application/json" } });
      projects = data.projects || [];
      document.getElementById("summary-total").textContent = String(data.summary?.registered ?? projects.length);
      document.getElementById("summary-profiles").textContent = String(new Set(projects.map((project) => project.profile)).size);
      document.getElementById("summary-evidence").textContent = String(data.summary?.with_evidence ?? 0);
      document.getElementById("updated-label").textContent = `Updated ${new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
      if (!projects.length) {
        document.getElementById("empty-state").hidden = false;
        document.getElementById("project-count").textContent = "No registered projects";
      } else {
        renderProjects(projects);
        document.querySelector(".table-scroll").hidden = false;
      }
      document.getElementById("project-search").addEventListener("input", () => renderProjects(projects));
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
