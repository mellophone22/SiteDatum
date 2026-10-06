document.documentElement.classList.add('motion-ready');

const navToggle = document.querySelector('[data-nav-toggle]');
const nav = document.querySelector('[data-nav]');

const closeNavigation = () => {
  navToggle?.setAttribute('aria-expanded', 'false');
  nav?.classList.remove('is-open');
};

navToggle?.addEventListener('click', () => {
  const nextExpanded = navToggle.getAttribute('aria-expanded') !== 'true';
  navToggle.setAttribute('aria-expanded', String(nextExpanded));
  nav?.classList.toggle('is-open', nextExpanded);
});

nav?.addEventListener('click', (event) => {
  if (event.target instanceof HTMLAnchorElement) closeNavigation();
});

window.addEventListener('resize', () => {
  if (window.innerWidth > 980) closeNavigation();
});

const revealItems = [...document.querySelectorAll('[data-reveal]')];
const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

if (!('IntersectionObserver' in window)) {
  revealItems.forEach((item) => item.classList.add('is-visible'));
} else {
  const revealObserver = new IntersectionObserver((entries, observer) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      entry.target.classList.add('is-visible');
      observer.unobserve(entry.target);
    });
  }, { threshold: reduceMotion ? 0.02 : 0.14, rootMargin: '0px 0px -7% 0px' });

  revealItems.forEach((item) => revealObserver.observe(item));
}

const sectionLinks = [...document.querySelectorAll('.primary-nav a[href^="#"]')];
const linkedSections = sectionLinks
  .map((link) => document.querySelector(link.getAttribute('href')))
  .filter(Boolean);

if ('IntersectionObserver' in window && linkedSections.length) {
  const sectionObserver = new IntersectionObserver((entries) => {
    const visible = entries
      .filter((entry) => entry.isIntersecting)
      .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
    if (!visible) return;
    sectionLinks.forEach((link) => {
      const current = link.getAttribute('href') === `#${visible.target.id}`;
      if (current) link.setAttribute('aria-current', 'true');
      else link.removeAttribute('aria-current');
    });
  }, { rootMargin: '-20% 0px -64% 0px', threshold: [0, 0.2, 0.6] });
  linkedSections.forEach((section) => sectionObserver.observe(section));
}

const screenshotDialog = document.querySelector('[data-screenshot-dialog]');
const screenshotDialogImage = document.querySelector('[data-screenshot-dialog-image]');

document.querySelectorAll('[data-screenshot-open]').forEach((trigger) => {
  trigger.addEventListener('click', () => {
    if (!(screenshotDialog instanceof HTMLDialogElement) || !(screenshotDialogImage instanceof HTMLImageElement)) return;
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
