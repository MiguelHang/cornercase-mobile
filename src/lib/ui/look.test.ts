import { describe, expect, it } from "vitest";
import buttonSource from "./Button.svelte?raw";
import emblemSource from "./Emblem.svelte?raw";
import glassSource from "./Glass.svelte?raw";

const flat = (text: string) => text.replace(/\s+/g, " ");

describe("Glass look", () => {
  it.each([
    "border: 1px solid var(--border);",
    "background: linear-gradient(180deg, #1c1f24e0, #101215e8);",
    "box-shadow: 0 24px 60px rgb(0 0 0 / 0.5), 0 4px 12px rgb(0 0 0 / 0.3);",
    "left: var(--corner); right: var(--corner); height: 1px;",
    "background: linear-gradient(90deg, rgb(236 238 241 / 0.18), rgb(236 238 241 / 0.02));",
  ])("holds %s", (text) => {
    expect(flat(glassSource)).toContain(text);
  });
});

describe("Button look", () => {
  it.each([
    "border: 1px solid rgb(43 123 255 / 0.7);",
    "border-radius: var(--radius-control);",
    "background: linear-gradient(180deg, var(--brand-light), var(--brand));",
    "box-shadow: 0 0 10px rgb(0 95 255 / 0.28);",
    "color: var(--on-brand);",
    "font-weight: 500;",
  ])("holds %s", (text) => {
    expect(flat(buttonSource)).toContain(text);
  });
});

describe("Emblem look", () => {
  it.each([
    "border: 1px solid rgb(43 123 255 / 0.35);",
    "background: rgb(0 95 255 / 0.16);",
    "color: var(--accent);",
    "gap: 10px;",
    "max-width: 470px;",
    "color: var(--muted);",
  ])("holds %s", (text) => {
    expect(flat(emblemSource)).toContain(text);
  });
});
