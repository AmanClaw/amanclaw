import { writable } from 'svelte/store';

type Theme = 'dark' | 'light';

function createThemeStore() {
  const stored = typeof localStorage !== 'undefined'
    ? localStorage.getItem('amanclaw-theme') as Theme | null
    : null;

  const systemPreference: Theme =
    typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: light)').matches
      ? 'light'
      : 'dark';

  // Default to dark — AmanClaw's brand is dark-first
  const initial = stored ?? 'dark';

  const { subscribe, set } = writable<Theme>(initial);

  function apply(theme: Theme) {
    const root = document.documentElement;
    root.classList.remove('dark', 'light');
    root.classList.add(theme);
    localStorage.setItem('amanclaw-theme', theme);
  }

  if (typeof document !== 'undefined') {
    apply(initial);
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
