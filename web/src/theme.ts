// Light/dark switch. Until the user picks one the page follows the system;
// the choice is then remembered on this device.

type Theme = "light" | "dark";

const STORAGE_KEY = "soap-theme";
const BAR_COLOR: Record<Theme, string> = { light: "#f1fbf8", dark: "#0f2429" };
const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

function storedTheme(): Theme | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    return null;
  }
}

export function currentTheme(): Theme {
  const forced = document.documentElement.dataset.theme;
  if (forced === "light" || forced === "dark") return forced;
  return systemDark.matches ? "dark" : "light";
}

/** Wires the toggle button; `onChange` runs after every switch (to repaint canvases). */
export function initThemeToggle(button: HTMLButtonElement, onChange: () => void) {
  const apply = (theme: Theme | null) => {
    if (theme) document.documentElement.dataset.theme = theme;
    else delete document.documentElement.dataset.theme;
    const dark = currentTheme() === "dark";
    button.setAttribute("aria-pressed", String(dark));
    button.querySelector<SVGElement>(".icon-moon")?.toggleAttribute("hidden", dark);
    button.querySelector<SVGElement>(".icon-sun")?.toggleAttribute("hidden", !dark);
    document.querySelectorAll('meta[name="theme-color"]').forEach((meta) => {
      if (theme) meta.setAttribute("content", BAR_COLOR[theme]);
      else meta.setAttribute("content", BAR_COLOR[meta.getAttribute("media")?.includes("dark") ? "dark" : "light"]);
    });
    onChange();
  };
  apply(storedTheme());
  systemDark.addEventListener("change", () => {
    if (!storedTheme()) apply(null);
  });
  button.addEventListener("click", () => {
    const next: Theme = currentTheme() === "dark" ? "light" : "dark";
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {}
    apply(next);
  });
}
