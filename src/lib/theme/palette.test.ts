import { describe, expect, it } from "vitest";
import base from "./base.css?raw";
import tokens from "./tokens.css?raw";

const value = (name: string) =>
  tokens.match(new RegExp(`${name}:\\s*([^;]+);`))?.[1];

describe("tokens", () => {
  it.each([
    ["--glass", "#0b0c0e"],
    ["--raised", "#1d2025"],
    ["--surface", "#2a2e354d"],
    ["--panel", "#ffffff0a"],
    ["--field", "#0506088c"],
    ["--veil", "#0a0b109e"],
    ["--border", "#ffffff17"],
    ["--edge", "#ffffff2b"],
    ["--hover", "#ffffff0f"],
    ["--selected", "#ffffff1a"],
    ["--text", "#eceef1"],
    ["--soft", "#b6bcc4"],
    ["--muted", "#8b929b"],
    ["--faint", "#6b727c"],
    ["--brand", "#005fff"],
    ["--brand-hover", "#0050d8"],
    ["--brand-light", "#2b7bff"],
    ["--on-brand", "#f7f8fa"],
    ["--accent", "#74a3ff"],
    ["--accent-soft", "#005fff2e"],
    ["--selection", "#005fff47"],
    ["--working", "#f5d76e"],
    ["--waiting", "#ff8700"],
    ["--done", "#58e6a0"],
    ["--failed", "#ff6b8b"],
    ["--merged", "#875fff"],
    ["--radius-chip", "5px"],
    ["--radius-control", "8px"],
    ["--radius-card", "12px"],
    ["--radius-sheet", "16px"],
  ])("%s is %s", (name, expected) => {
    expect(value(name)).toBe(expected);
  });

  it("names a UI and a mono font family", () => {
    expect(value("--font-ui")).toContain("system-ui");
    expect(value("--font-mono")).toContain("ui-monospace");
  });
});

describe("base styles", () => {
  it.each([
    "background: var(--glass);",
    "color-scheme: dark;",
    "font-family: var(--font-ui);",
    "-webkit-tap-highlight-color: transparent;",
    "-webkit-user-select: none;",
    "user-select: none;",
    "-webkit-touch-callout: none;",
    "-webkit-text-size-adjust: 100%;",
    "env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom)",
    "env(safe-area-inset-left)",
  ])("holds %s", (text) => {
    expect(base.replace(/\s+/g, " ")).toContain(text);
  });

  it("keeps the brand wash fixed, 180 px tall, fading from 12 % to 0", () => {
    const wash = base
      .match(/body::before \{([^}]*)\}/)?.[1]
      .replace(/\s+/g, " ");
    expect(wash).toContain("position: fixed;");
    expect(wash).toContain("height: 180px;");
    expect(wash).toContain(
      "linear-gradient(180deg, rgb(0 95 255 / 0.12), rgb(0 95 255 / 0))",
    );
  });
});
