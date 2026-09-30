import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { appearanceStorageKey, readAppearanceMode, resolveTheme, type AppearanceMode, type ResolvedTheme } from "./themeState";

type ThemeValue = {
  mode: AppearanceMode;
  resolvedTheme: ResolvedTheme;
  setMode: (mode: AppearanceMode) => void;
  toggleTheme: () => void;
};

const ThemeContext = createContext<ThemeValue | null>(null);

const systemQuery = () => window.matchMedia("(prefers-color-scheme: dark)");

function applyTheme(theme: ResolvedTheme) {
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
}

export function initializeTheme() {
  const mode = readAppearanceMode(localStorage);
  applyTheme(resolveTheme(mode, systemQuery().matches));
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<AppearanceMode>(() => readAppearanceMode(localStorage));
  const [systemDark, setSystemDark] = useState(() => systemQuery().matches);
  const resolvedTheme = resolveTheme(mode, systemDark);

  useEffect(() => {
    const query = systemQuery();
    const update = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);

  useEffect(() => { applyTheme(resolvedTheme); }, [resolvedTheme]);

  const value = useMemo<ThemeValue>(() => ({
    mode,
    resolvedTheme,
    setMode: (next) => { localStorage.setItem(appearanceStorageKey, next); setModeState(next); },
    toggleTheme: () => {
      const next = resolvedTheme === "dark" ? "light" : "dark";
      localStorage.setItem(appearanceStorageKey, next);
      setModeState(next);
    },
  }), [mode, resolvedTheme]);

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("useTheme must be used within ThemeProvider");
  return value;
}

export function ThemeToggle() {
  const { mode, resolvedTheme, toggleTheme } = useTheme();
  const dark = resolvedTheme === "dark";
  const source = mode === "system" ? "Windows setting" : `${resolvedTheme} mode`;
  return <button type="button" className="theme-toggle" role="switch" aria-checked={dark} aria-label={`Dark mode. ${dark ? "On" : "Off"}. Using ${source}.`} title={`Switch to ${dark ? "light" : "dark"} mode`} onClick={toggleTheme}>
    <svg className="theme-sun" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3.5"/><path d="M12 2.5v2M12 19.5v2M4.6 4.6 6 6M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4"/></svg>
    <span className="theme-track" aria-hidden="true"><i /></span>
    <svg className="theme-moon" viewBox="0 0 24 24" aria-hidden="true"><path d="M19.5 15.2A8 8 0 0 1 8.8 4.5 8 8 0 1 0 19.5 15.2Z"/></svg>
  </button>;
}

const options: Array<{ value: AppearanceMode; label: string; detail: string }> = [
  { value: "light", label: "Light", detail: "Bright working canvas" },
  { value: "dark", label: "Dark", detail: "Low-light working canvas" },
  { value: "system", label: "Windows default", detail: "Follows this computer automatically" },
];

export function AppearanceSettings() {
  const { mode, resolvedTheme, setMode } = useTheme();
  return <div className="appearance-options" role="radiogroup" aria-label="Application appearance">
    {options.map(option => <button key={option.value} type="button" role="radio" aria-checked={mode === option.value} className={mode === option.value ? "appearance-option selected" : "appearance-option"} onClick={() => setMode(option.value)}>
      <span className={`appearance-preview ${option.value}`} aria-hidden="true"><i /><i /><i /></span>
      <span><strong>{option.label}</strong><small>{option.detail}</small></span>
      <span className="appearance-check" aria-hidden="true">{mode === option.value ? "✓" : ""}</span>
    </button>)}
    <p className="appearance-current" aria-live="polite">Currently displaying <strong>{resolvedTheme} mode</strong>{mode === "system" ? " from Windows." : "."}</p>
  </div>;
}
