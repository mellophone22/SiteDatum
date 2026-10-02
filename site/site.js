document.documentElement.classList.add('motion-ready');

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
let workflowSequence = 0;

workflowTabs.forEach((tab) => {
  tab.addEventListener('click', async () => {
    const sequence = ++workflowSequence;
    const key = tab.getAttribute('data-workflow-tab');
    const content = key ? workflowContent[key] : undefined;
    if (!content || !workflowPanel) return;
    workflowTabs.forEach((item) => {
      const active = item === tab;
      item.classList.toggle('is-active', active);
      item.setAttribute('aria-pressed', String(active));
    });
    const image = workflowPanel.querySelector('[data-workflow-image]');
    const trigger = workflowPanel.querySelector('[data-screenshot-open]');
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!reduceMotion && image.animate) {
      try {
        image.getAnimations().forEach((animation) => animation.cancel());
        await image.animate(
          [{ opacity: 1, clipPath: 'inset(0)' }, { opacity: 0, clipPath: 'inset(0 0 0 14%)' }],
          { duration: 130, easing: 'ease-in', fill: 'forwards' },
        ).finished;
      } catch {
        // A newer tab choice interrupted this transition; continue with its content.
      }
    }
    if (sequence !== workflowSequence) return;
    workflowPanel.querySelector('[data-workflow-title]').textContent = content.title;
    workflowPanel.querySelector('[data-workflow-copy]').textContent = content.copy;
    const list = workflowPanel.querySelector('[data-workflow-list]');
    list.replaceChildren(...content.items.map((item) => {
      const li = document.createElement('li');
      li.textContent = item;
      return li;
    }));
    image.src = content.image;
    image.alt = content.alt;
    trigger.dataset.fullImage = content.image;
    trigger.dataset.fullAlt = content.alt;
    trigger.setAttribute('aria-label', `Open ${content.alt} at full resolution`);
    if (!reduceMotion && image.animate) {
      image.animate(
        [{ opacity: 0, clipPath: 'inset(0 14% 0 0)' }, { opacity: 1, clipPath: 'inset(0)' }],
        { duration: 280, easing: 'cubic-bezier(.16,1,.3,1)', fill: 'both' },
      );
    }
  });
});

const screenshotDialog = document.querySelector('[data-screenshot-dialog]');
const screenshotDialogImage = document.querySelector('[data-screenshot-dialog-image]');

document.querySelectorAll('[data-screenshot-open]').forEach((trigger) => {
  trigger.addEventListener('click', () => {
    screenshotDialogImage.src = trigger.dataset.fullImage;
    screenshotDialogImage.alt = trigger.dataset.fullAlt;
    screenshotDialog.showModal();
  });
});

document.querySelector('[data-screenshot-close]')?.addEventListener('click', () => screenshotDialog?.close());
screenshotDialog?.addEventListener('click', (event) => {
  if (event.target === screenshotDialog) screenshotDialog.close();
});

document.querySelectorAll('[data-year]').forEach((element) => {
  element.textContent = String(new Date().getFullYear());
});

