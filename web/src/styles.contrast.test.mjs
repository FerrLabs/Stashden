import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const css = readFileSync(new URL('./styles.css', import.meta.url), 'utf8');

const TEXT_MINIMUM = 4.5;
const COMPONENT_MINIMUM = 3;
const HOVER_BRIGHTNESS = 1.06;

const textPairs = [
  ['--ink', ['--page', '--surface', '--surface-raised']],
  ['--ink-muted', ['--page', '--surface', '--surface-raised']],
  ['--ink-faint', ['--page', '--surface', '--surface-raised', '--rule-soft']],
  ['--accent-ink', ['--page', '--surface', '--surface-raised', '--accent-wash']],
  ['--danger-ink', ['--surface', '--danger-wash']],
  ['--accent-contrast', ['--accent-ink']],
];

const focusRingPairs = [['--accent', ['--page', '--surface', '--surface-raised']]];

function declarations(block) {
  return Object.fromEntries(
    [...block.matchAll(/(--[\w-]+):\s*(#[0-9a-f]{6})\s*;/gi)].map(([, name, value]) => [name, value]),
  );
}

function themes() {
  const dark = css.match(/@media \(prefers-color-scheme: dark\) \{\s*:root \{([^}]*)\}/)[1];
  const light = css.match(/:root \{([^}]*)\}/)[1];
  return { light: declarations(light), dark: { ...declarations(light), ...declarations(dark) } };
}

function channels(hex) {
  return [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16));
}

function luminance(channelValues) {
  const [red, green, blue] = channelValues.map((value) => {
    const unit = value / 255;
    return unit <= 0.04045 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function ratio(first, second) {
  const [lighter, darker] = [luminance(first), luminance(second)].sort((a, b) => b - a);
  return (lighter + 0.05) / (darker + 0.05);
}

function brightened(hex) {
  return channels(hex).map((value) => Math.min(255, Math.round(value * HOVER_BRIGHTNESS)));
}

function check(pairs, minimum) {
  for (const [themeName, tokens] of Object.entries(themes())) {
    for (const [foreground, backgrounds] of pairs) {
      for (const background of backgrounds) {
        const value = ratio(channels(tokens[foreground]), channels(tokens[background]));
        assert.ok(
          value >= minimum,
          `${themeName}: ${foreground} on ${background} is ${value.toFixed(2)}:1, needs ${minimum}:1`,
        );
      }
    }
  }
}

test('text tokens reach 4.5:1 on the backgrounds they are drawn on', () => {
  check(textPairs, TEXT_MINIMUM);
});

test('focus ring reaches 3:1 on the page and surfaces', () => {
  check(focusRingPairs, COMPONENT_MINIMUM);
});

test('primary button label keeps 4.5:1 while hovered', () => {
  for (const [themeName, tokens] of Object.entries(themes())) {
    const value = ratio(channels(tokens['--accent-contrast']), brightened(tokens['--accent-ink']));
    assert.ok(value >= TEXT_MINIMUM, `${themeName}: hovered primary button is ${value.toFixed(2)}:1`);
  }
});

test('primary button fill is at least 3:1 against the surface it sits on', () => {
  check([['--accent-ink', ['--page', '--surface']]], COMPONENT_MINIMUM);
});
