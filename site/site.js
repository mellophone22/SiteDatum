const navToggle = document.querySelector('[data-nav-toggle]');
const nav = document.querySelector('[data-nav]');

navToggle?.addEventListener('click', () => {
  const expanded = navToggle.getAttribute('aria-expanded') === 'true';
  navToggle.setAttribute('aria-expanded', String(!expanded));
  nav?.classList.toggle('is-open', !expanded);
});

nav?.addEventListener('click', (event) => {
  if (event.target instanceof HTMLAnchorElement) {
    navToggle?.setAttribute('aria-expanded', 'false');
    nav.classList.remove('is-open');
  }
});

const workflowContent = {
  attention: {
    title: 'Know what needs you now',
    copy: 'Open one queue for overdue work, due-soon commitments, blocked items, and responses waiting on others.',
    items: ['Portfolio and project-specific attention', 'Plain-language reasons and dates', 'Direct routes to the underlying record'],
    image: 'assets/images/task-management.png',
    alt: 'SiteDatum task register',
  },
  context: {
    title: 'Move without losing context',
    copy: 'Open the project and act with its tasks, RFIs, submittals, files, notes, contacts, and operational registers close at hand.',
    items: ['Project-centered navigation', 'Related records one action away', 'Quick capture from anywhere'],
    image: 'assets/images/rfi-register.png',
    alt: 'SiteDatum RFI register',
  },
  record: {
    title: 'Leave the record clearer',
    copy: 'Update status, preserve relationships, and keep normal Windows documents accessible outside the application.',
    items: ['Collision-safe file operations', 'Searchable activity and recovery', 'Exports and backups remain available'],
    image: 'assets/images/portfolio-overview.png',
    alt: 'SiteDatum portfolio overview',
  },
};

const workflowTabs = document.querySelectorAll('[data-workflow-tab]');
const workflowPanel = document.querySelector('[data-workflow-panel]');

workflowTabs.forEach((tab) => {
  tab.addEventListener('click', () => {
    const key = tab.getAttribute('data-workflow-tab');
    const content = key ? workflowContent[key] : undefined;
    if (!content || !workflowPanel) return;
    workflowTabs.forEach((item) => {
      const active = item === tab;
      item.classList.toggle('is-active', active);
      item.setAttribute('aria-pressed', String(active));
    });
    workflowPanel.querySelector('[data-workflow-title]').textContent = content.title;
    workflowPanel.querySelector('[data-workflow-copy]').textContent = content.copy;
    const list = workflowPanel.querySelector('[data-workflow-list]');
    list.replaceChildren(...content.items.map((item) => {
      const li = document.createElement('li');
      li.textContent = item;
      return li;
    }));
    const image = workflowPanel.querySelector('[data-workflow-image]');
    image.src = content.image;
    image.alt = content.alt;
  });
});

document.querySelectorAll('[data-year]').forEach((element) => {
  element.textContent = String(new Date().getFullYear());
});

