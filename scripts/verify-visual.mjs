/* global process */
// Isolated browser verification. Fixtures never enter the shipped application or user database.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdir } from "node:fs/promises";
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const { version } = require("../package.json");
const output = "tmp/visual-review";
await mkdir(output, { recursive: true });

const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  await context.addInitScript(({ version }) => {
    localStorage.setItem("workspace.screen", "overview");
    const names = ["Riverside Medical Center", "Westhaven Campus", "Harbor Operations Center", "Northside Science Building", "Cypress Elementary"];
    const projects = Array.from({ length: 25 }, (_, i) => ({
      id: `project-${i}`, number: `26-${String(104 + i)}`, name: names[i % names.length] + (i > 4 ? ` / Phase ${Math.floor(i / 5) + 1}` : ""),
      status: "active", phase: ["engineering", "construction", "commissioning"][i % 3], customPhaseName: null,
      customer: ["County Facilities", "Regional Health", "Education District"][i % 3], targetDate: "2026-12-15",
      isPinned: i === 0, archivedAtUtc: null, projectPath: `C:\\VisualFixtures\\26-${104 + i}`,
    }));
    const titles = ["Confirm AHU-03 sequence with engineer", "Review chilled-water valve submittal", "Coordinate roof sensor locations", "Issue updated network riser", "Verify field controller point counts", "Close out startup deficiencies", "Review air balance report", "Confirm equipment delivery dates"];
    const tasks = Array.from({ length: 240 }, (_, i) => ({
      id: `task-${i}`, projectId: projects[i % 25].id, projectNumber: projects[i % 25].number, projectName: projects[i % 25].name,
      title: titles[i % titles.length] + (i > 7 ? ` / Area ${Math.floor(i / 8) + 1}` : ""),
      status: ["open", "in_progress", "waiting", "completed", "blocked"][i % 5], priority: ["high", "medium", "low"][i % 3],
      dueDate: "2026-09-28", followUpDate: i % 5 === 2 ? "2026-09-25" : null,
      waitingOn: i % 5 === 2 ? "Design engineer" : null, category: "Coordination", description: "Confirm requirements against the latest approved drawing set.",
    }));
    const rfis = [{ id: "rfi-1", projectId: "project-0", projectNumber: "26-104", projectName: names[0], number: "RFI-018", subject: "Smoke control interface requirements", recipient: "Design engineer", responseDueDate: "2026-09-28", status: "open", relatedTaskId: null }];
    const submittals = [{ id: "sub-1", projectId: "project-1", projectNumber: "26-105", projectName: names[1], number: "SUB-006", name: "DDC controls package", recipient: "Consultant", status: "under_review", revision: "0", submittedDate: "2026-09-20", relatedTaskId: null }];
    const queues = ["overdue", "today", "follow_up", "waiting", "upcoming"].map((key, i) => ({ key, label: ["Overdue", "Today", "Follow-up", "Waiting", "Upcoming"][i], tasks: tasks.filter(t => t.status !== "completed").slice(i * 3, i * 3 + 3) }));
    const commands = {
      get_project_root: { path: "C:\\VisualFixtures" }, get_cloud_auth_status: { connected: false, email: null },
      "plugin:dialog|open": "\\\\?\\C:\\VisualFixtures",
      validate_project_root_command: { canonicalPath: "C:\\VisualFixtures", pathKind: "local", warning: null },
      save_project_root: { path: "C:\\VisualFixtures" },
      list_projects: projects, list_tasks: tasks, list_attention: queues, list_rfis: rfis, list_rfi_attention: rfis,
      list_submittals: submittals, list_submittal_attention: submittals, list_files: [], list_contacts: [], list_cloud_conflicts: [], list_project_templates: [], list_local_backups: [],
      list_work_items: [{ id: "milestone-1", projectId: "project-0", projectNumber: "26-104", projectName: names[0], itemType: "milestone", title: "Controls rough-in complete", status: "open", dueDate: "2026-10-01", priority: "medium" }],
      list_notes: [{ id: "note-1", projectId: "project-0", projectNumber: "26-104", projectName: names[0], body: "Coordinate final controller locations with the electrical contractor before ceiling close-in.", createdAtUtc: "2026-09-25T15:00:00Z" }],
      list_activity: [{ eventType: "created", entityType: "rfi", summary: "RFI-018 recorded for Riverside Medical Center", occurredAtUtc: "2026-09-25T14:30:00Z" }],
      "plugin:app|version": version,
    };
    window.isTauri = true;
    window.__TAURI_INTERNALS__ = { invoke: async (command) => {
      if (Object.hasOwn(commands, command)) return structuredClone(commands[command]);
      throw new Error(`Unmocked visual-test command: ${command}`);
    } };
  }, { version });
  const page = await context.newPage();
  await page.emulateMedia({ colorScheme: "light" });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
  const go = async name => {
    await page.getByRole("button", { name, exact: true }).first().click();
    await page.waitForFunction(() => !document.querySelector(".loading-state"));
  };
  const capture = async name => page.screenshot({ path: `${output}/${name}.png`, animations: "disabled" });
  const noOverflow = async () => {
    const dimensions = await page.evaluate(() => ({ scrollWidth: document.documentElement.scrollWidth, innerWidth, heading: document.querySelector("h1")?.textContent }));
    assert(dimensions.scrollWidth <= dimensions.innerWidth, `${dimensions.heading}: page width ${dimensions.scrollWidth}px exceeds viewport ${dimensions.innerWidth}px`);
  };
  await page.goto("http://localhost:1420");
  await page.getByRole("heading", { name: "All active projects" }).waitFor();
  await capture("home-desktop");
  await noOverflow();
  assert.equal(await page.locator("vite-error-overlay").count(), 0);
  const themeToggle = page.getByRole("switch", { name: /Dark mode/ });
  assert.equal(await themeToggle.getAttribute("aria-checked"), "false");
  await themeToggle.click();
  await page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
  assert.equal(await page.evaluate(() => localStorage.getItem("appearance.mode")), "dark");
  await capture("home-dark");
  for (const name of ["Attention", "Projects", "Tasks", "RFIs", "Submittals", "Files", "Project Controls", "Notes & Contacts", "Recovery", "About SiteDatum"]) {
    await go(name);
    await noOverflow();
    assert.equal(await page.getByRole("alert").count(), 0, `${name}: no dark-mode error notices`);
    if (["Tasks", "Project Controls", "About SiteDatum"].includes(name)) await capture(`${name.toLowerCase().replaceAll(/[^a-z]+/g, "-")}-dark`);
  }
  await go("Settings");
  assert.equal(await page.getByRole("radio", { name: /Dark/ }).getAttribute("aria-checked"), "true");
  await capture("settings-dark");
  await page.getByRole("radio", { name: /Windows default/ }).click();
  assert.equal(await page.evaluate(() => localStorage.getItem("appearance.mode")), "system");
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
  await page.emulateMedia({ colorScheme: "light" });
  await page.waitForFunction(() => document.documentElement.dataset.theme === "light");
  await capture("settings-light");
  await page.getByRole("button", { name: "Browse…", exact: true }).click();
  assert.equal(await page.locator("#project-root").inputValue(), "C:\\VisualFixtures");
  await page.getByRole("button", { name: "Check location", exact: true }).click();
  await page.getByText("Local folder is available.", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Save project root", exact: true }).click();
  await page.getByText("Project root saved and ready for project workspaces.", { exact: true }).waitFor();
  await page.reload();
  await page.waitForFunction(() => document.documentElement.dataset.theme === "light");
  assert.equal(await page.evaluate(() => localStorage.getItem("appearance.mode")), "system");
  await go("Home");
  await go("About SiteDatum");
  await page.getByText(`Version ${version}`, { exact: true }).waitFor();
  assert.equal(await page.getByText("Created by Francisco Cabrera", { exact: true }).count(), 1);
  assert.equal(await page.locator(".about-statement").innerText(), "The project record\nyou control");
  assert.equal(await page.locator(".about-brand .brand-mark").evaluate(img => img.complete && img.naturalWidth > 0), true);
  await capture("about-desktop");
  await go("Tasks");
  await page.locator("tbody tr").nth(239).waitFor();
  await capture("tasks-desktop");
  await page.locator("tbody .project-link").first().click();
  await page.getByRole("heading", { name: titlesForTest() }).waitFor();
  await capture("task-detail");
  await page.keyboard.press("Escape");
  assert.equal(await page.locator(".detail-panel").count(), 0);
  assert.equal(await page.evaluate(() => document.activeElement.classList.contains("project-link")), true);
  await page.getByRole("button", { name: "New task", exact: true }).click();
  assert.equal(await page.getByRole("button", { name: "Add task", exact: true }).isDisabled(), true);
  await capture("task-create");
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await go("Projects");
  assert.equal(await page.locator("tbody tr").count(), 25);
  await capture("projects-desktop");
  await page.getByRole("combobox", { name: "Current project", exact: true }).selectOption("project-0");
  await go("About SiteDatum");
  assert.equal(await page.getByRole("combobox", { name: "Current project", exact: true }).inputValue(), "project-0");
  await page.getByRole("button", { name: "Search workspace", exact: true }).click();
  await page.getByRole("textbox", { name: "Search local records" }).fill("version");
  await page.getByRole("option", { name: /About SiteDatum/ }).click();
  assert.equal(await page.getByRole("dialog").count(), 0);
  await page.getByRole("button", { name: /Quick capture/ }).click();
  await page.waitForFunction(() => document.querySelector(".quick-capture select")?.value === "project-0");
  await capture("quick-capture");
  await page.keyboard.press("Escape");
  assert.equal(await page.getByRole("dialog").count(), 0);
  await page.getByRole("combobox", { name: "Current project", exact: true }).selectOption("");
  for (const name of ["Attention", "RFIs", "Submittals", "Files", "Project Controls", "Notes & Contacts", "Settings", "Recovery"]) {
    await go(name);
    await capture(name.toLowerCase().replaceAll(/[^a-z]+/g, "-"));
    await noOverflow();
    assert.equal(await page.getByRole("alert").count(), 0, `${name}: no error notices`);
  }
  await page.setViewportSize({ width: 720, height: 900 });
  for (const name of ["Home", "Tasks", "Projects", "About SiteDatum", "Settings"]) {
    await go(name);
    await noOverflow();
    await capture(`${name.toLowerCase().replaceAll(/[^a-z]+/g, "-")}-narrow`);
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  await go("About SiteDatum");
  assert.equal(await page.locator(".about-page").evaluate(el => getComputedStyle(el).animationName), "none");
  await page.setViewportSize({ width: 1280, height: 720 });
  await capture("about-short-window");
  assert(await page.locator(".main-navigation").evaluate(el => el.scrollHeight <= el.clientHeight), "Navigation fits a 720px-high window");
  for (const viewport of [{ width: 960, height: 680 }, { width: 760, height: 540 }]) {
    await page.setViewportSize(viewport);
    for (const name of ["Home", "Tasks", "About SiteDatum"]) { await go(name); await noOverflow(); }
    await capture(`about-${viewport.width}`);
  }
  await page.setViewportSize({ width: 640, height: 800 });
  for (const name of ["Home", "Tasks", "About SiteDatum"]) { await go(name); await noOverflow(); }
  await page.goto("http://localhost:1420/?first-run-preview");
  await page.getByRole("heading", { name: "Choose your project root" }).waitFor();
  await capture("first-run-narrow");
  assert(await page.getByRole("img", { name: "SiteDatum", exact: true }).isVisible());
  await noOverflow();
  assert.deepEqual(errors, []);
  console.log("PASS: 25 projects / 240 tasks; all reviewed routes; About content/version/logo; search; project preservation; detail Escape/focus; disabled create; Quick Capture; desktop/narrow overflow; reduced motion; no console errors.");
  await context.close();
} finally {
  await browser.close();
}

function titlesForTest() { return /Confirm AHU-03 sequence with engineer/; }
