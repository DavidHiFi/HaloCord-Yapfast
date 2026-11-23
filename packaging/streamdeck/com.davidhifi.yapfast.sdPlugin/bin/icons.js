'use strict';
// The keys' pictures: one Mocha line glyph and a short word, drawn as SVG so
// the plugin can redraw a key whenever HaloCord's state changes. The glyphs
// follow Lucide's 24-unit grid (ISC licence, lucide.dev).

const MOCHA = {
  base: '#1E1E2E',
  surface0: '#313244',
  surface1: '#45475A',
  overlay0: '#6C7086',
  subtext0: '#A6ADC8',
  text: '#CDD6F4',
  blue: '#89B4FA',
  mauve: '#CBA6F7',
  red: '#F38BA8',
};

const GLYPHS = {
  mic: '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><path d="M12 19v3"/>',
  micOff: '<path d="M2 2l20 20"/><path d="M18.89 13.23A7.12 7.12 0 0 0 19 12v-2"/><path d="M5 10v2a7 7 0 0 0 12 5"/><path d="M15 9.34V5a3 3 0 0 0-5.68-1.33"/><path d="M9 9v3a3 3 0 0 0 5.12 2.12"/><path d="M12 19v3"/>',
  headphones: '<path d="M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3"/>',
  headphonesOff: '<path d="M21 14h-1.34"/><path d="M9.13 3.47A9 9 0 0 1 21 12v3.34"/><path d="M2 2l20 20"/><path d="M20.41 20.41A2 2 0 0 1 19 21h-1a2 2 0 0 1-2-2v-3"/><path d="M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 2.64-6.36"/>',
  camera: '<path d="M16 13l5.22 3.48a.5.5 0 0 0 .78-.42V7.87a.5.5 0 0 0-.75-.43L16 10.5"/><rect x="2" y="6" width="14" height="12" rx="2"/>',
  cameraOff: '<path d="M10.66 6H14a2 2 0 0 1 2 2v2.5l5.25-3.06a.5.5 0 0 1 .75.43v8.2"/><path d="M16 16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2"/><path d="M2 2l20 20"/>',
  share: '<path d="M13 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-3"/><path d="M8 21h8"/><path d="M12 17v4"/><path d="M17 8l5-5"/><path d="M17 3h5v5"/>',
  shareStop: '<path d="M13 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-3"/><path d="M8 21h8"/><path d="M12 17v4"/><path d="M22 3l-5 5"/><path d="M17 3l5 5"/>',
  screen: '<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/>',
  voiceOpen: '<path d="M2 10v3"/><path d="M6 6v11"/><path d="M10 3v18"/><path d="M14 8v7"/><path d="M18 5v13"/><path d="M22 10v3"/>',
  voicePush: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M12 8v7"/><path d="M8.5 11.5L12 15l3.5-3.5"/>',
  leave: '<path d="M10.1 13.9a14 14 0 0 0 3.73 2.67 1 1 0 0 0 1.21-.3l.36-.47A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2 18 18 0 0 1-12.73-5.27"/><path d="M22 2L2 22"/><path d="M4.76 13.58A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.47.35a1 1 0 0 0-.29 1.23 14 14 0 0 0 .24.47"/>',
};

const TONES = {
  rest: { glyph: MOCHA.text, label: MOCHA.subtext0 },
  dim: { glyph: MOCHA.overlay0, label: MOCHA.overlay0 },
  faint: { glyph: MOCHA.surface1, label: MOCHA.surface1 },
  red: { glyph: MOCHA.red, label: MOCHA.red },
  live: { glyph: 'url(#glyph)', label: MOCHA.mauve },
};

const FONT = "'FiraCode Nerd Font', 'Segoe UI', sans-serif";

/**
 * One key, 144 units square.
 * @param {{glyph: string, label?: string, tone?: string, ring?: string|null, digit?: string, offline?: boolean}} look
 */
function key(look) {
  const tone = TONES[look.tone] || TONES.rest;
  const ring = look.ring === 'live' ? 'url(#ring)' : look.ring === 'red' ? MOCHA.red : MOCHA.surface0;
  const digit = look.digit
    ? `<text x="12" y="12.6" font-family="${FONT}" font-size="8" font-weight="700" text-anchor="middle" fill="${tone.glyph}" stroke="none">${look.digit}</text>`
    : '';
  const label = look.label
    ? `<text x="72" y="126" font-family="${FONT}" font-size="19" font-weight="700" text-anchor="middle" fill="${tone.label}">${look.label}</text>`
    : '';
  const offline = look.offline ? `<circle cx="118" cy="26" r="9" fill="${MOCHA.red}"/>` : '';
  const top = look.label ? 18 : 32;
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">',
    '<defs>',
    `<linearGradient id="ring" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="144" y2="144"><stop offset="0" stop-color="${MOCHA.blue}"/><stop offset="1" stop-color="${MOCHA.mauve}"/></linearGradient>`,
    `<linearGradient id="glyph" gradientUnits="userSpaceOnUse" x1="2" y1="2" x2="22" y2="22"><stop offset="0" stop-color="${MOCHA.blue}"/><stop offset="1" stop-color="${MOCHA.mauve}"/></linearGradient>`,
    '</defs>',
    `<rect x="0" y="0" width="144" height="144" fill="${MOCHA.base}"/>`,
    `<rect x="5" y="5" width="134" height="134" rx="24" ry="24" fill="none" stroke="${ring}" stroke-width="6"/>`,
    `<g transform="translate(40 ${top}) scale(2.6667)" fill="none" stroke="${tone.glyph}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${GLYPHS[look.glyph]}${digit}</g>`,
    label,
    offline,
    '</svg>',
  ].join('');
}

/** The small white picture in Stream Deck's action list. */
function listIcon(glyph, digit) {
  const number = digit
    ? `<text x="12" y="12.6" font-family="${FONT}" font-size="8" font-weight="700" text-anchor="middle" fill="#FFFFFF" stroke="none">${digit}</text>`
    : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${GLYPHS[glyph]}${number}</svg>`;
}

function dataUri(svg) {
  return 'data:image/svg+xml;base64,' + Buffer.from(svg, 'utf8').toString('base64');
}

module.exports = { key, listIcon, dataUri, GLYPHS };
