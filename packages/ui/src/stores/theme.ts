import { writable } from 'svelte/store';
import { browser } from '$app/environment';

type Theme = 'dark' | 'light';

function createThemeStore() {
  const stored = browser
    ? localStorage.getItem('amanclaw-theme') as Theme | null
    : null;

  // Default to light mode. User can toggle to dark via the theme button.
  const initial: Theme = stored ?? 'light';

  const { subscribe, set } = writable<Theme>(initial);

  function apply(theme: Theme) {
    if (!browser) return;
    const root = document.documentElement;
    root.classList.remove('dark', 'light');
    root.classList.add(theme);
    localStorage.setItem('amanclaw-theme', theme);
  }

  return {
    subscribe,
    toggle() {
      let current: Theme = 'dark';
      subscribe(v => current = v)();
      const next = current === 'dark' ? 'light' : 'dark';
      set(next);
      apply(next);
    },
    set(theme: Theme) {
      set(theme);
      apply(theme);
    }
  };
}

export const theme = createThemeStore();
