'use strict';
// Every key the plugin offers: what a press asks HaloCord for, and how the
// key looks for a given state from HaloCord's `/state`.

const SIDES = ['LEFT', 'MAIN', 'RIGHT', 'TOP'];

/** In a call or not, everything else off: what the manifest's pictures show. */
const RESTING = {
  ok: true, inCall: true, ready: true, mute: false, deafen: false, camera: false, canFilm: true,
  sharing: false, canShare: true, fakeMute: false, fakeDeafen: false, pttMode: false,
  talking: false, muteHeld: false,
  screens: [1, 2, 3, 4].map((slot) => ({ slot, present: true, active: false })),
};

const callTone = (s) => (s.inCall ? 'rest' : 'dim');

function screenKey(slot) {
  return {
    id: `screen${slot}`,
    name: `Screen ${slot} (${SIDES[slot - 1][0]}${SIDES[slot - 1].slice(1).toLowerCase()})`,
    tooltip: 'Share this screen in the call. Press another screen key to jump to it, or this one again to stop.',
    press: `screen-${slot}`,
    look(s) {
      const screen = (s.screens || []).find((each) => each.slot === slot) || { present: false };
      const base = { glyph: 'screen', digit: String(slot) };
      if (!screen.present) return { ...base, label: 'NONE', tone: 'faint', state: 0 };
      if (screen.active) return { ...base, label: 'LIVE', tone: 'live', ring: 'live', state: 1 };
      return { ...base, label: SIDES[slot - 1], tone: callTone(s), state: 0 };
    },
    on: { inCall: true, sharing: true, screens: RESTING.screens.map((each) => ({ ...each, active: each.slot === slot })) },
  };
}

const ACTIONS = [
  {
    id: 'mute', name: 'Mute', tooltip: 'Mute or unmute your microphone', press: 'mute',
    look: (s) => (s.mute
      ? { glyph: 'micOff', label: 'MUTED', tone: 'red', ring: 'red', state: 1 }
      : { glyph: 'mic', label: 'MIC', state: 0 }),
    on: { mute: true },
  },
  {
    id: 'deafen', name: 'Deafen', tooltip: 'Deafen or undeafen', press: 'deafen',
    look: (s) => (s.deafen
      ? { glyph: 'headphonesOff', label: 'DEAF', tone: 'red', ring: 'red', state: 1 }
      : { glyph: 'headphones', label: 'SOUND', state: 0 }),
    on: { deafen: true },
  },
  {
    id: 'camera', name: 'Camera', tooltip: 'Turn your camera on or off in the call', press: 'camera',
    look: (s) => (s.camera
      ? { glyph: 'camera', label: 'CAMERA', tone: 'live', ring: 'live', state: 1 }
      : { glyph: 'cameraOff', label: 'CAMERA', tone: s.inCall && s.canFilm ? 'rest' : 'dim', state: 0 }),
    on: { camera: true },
  },
  {
    id: 'share', name: 'Share Screen', tooltip: 'Open the share picker, or stop sharing', press: 'share',
    look: (s) => (s.sharing
      ? { glyph: 'share', label: 'LIVE', tone: 'live', ring: 'live', state: 1 }
      : { glyph: 'share', label: 'SHARE', tone: s.inCall && s.canShare ? 'rest' : 'dim', state: 0 }),
    on: { sharing: true },
  },
  {
    id: 'voicemode', name: 'Voice Mode', tooltip: 'Switch between voice activity and push to talk', press: 'voice-mode',
    look: (s) => (s.pttMode
      ? { glyph: 'voicePush', label: 'PUSH', tone: 'live', ring: 'live', state: 1 }
      : { glyph: 'voiceOpen', label: 'VOICE', state: 0 }),
    on: { pttMode: true },
  },
  {
    id: 'talk', name: 'Push to Talk', tooltip: 'Hold to talk', hold: 'talk',
    look: (s) => (s.talking
      ? { glyph: 'mic', label: 'TALKING', tone: 'live', ring: 'live', state: 1 }
      : { glyph: 'mic', label: 'TALK', tone: callTone(s), state: 0 }),
    on: { talking: true },
  },
  {
    id: 'mutehold', name: 'Push to Mute', tooltip: 'Hold to stay muted', hold: 'mute-hold',
    look: (s) => (s.muteHeld
      ? { glyph: 'micOff', label: 'MUTED', tone: 'red', ring: 'red', state: 1 }
      : { glyph: 'micOff', label: 'HOLD', tone: callTone(s), state: 0 }),
    on: { muteHeld: true },
  },
  {
    id: 'leave', name: 'Leave Call', tooltip: 'Leave the voice call', press: 'leave',
    look: (s) => (s.inCall
      ? { glyph: 'leave', label: 'LEAVE', tone: 'red', state: 1 }
      : { glyph: 'leave', label: 'NO CALL', tone: 'dim', state: 0 }),
    off: { inCall: false },
  },
  screenKey(1),
  screenKey(2),
  screenKey(3),
  screenKey(4),
  {
    id: 'fakemute', name: 'Fake Mute', tooltip: 'Look muted to others while you still talk', press: 'fake-mute',
    look: (s) => (s.fakeMute
      ? { glyph: 'micOff', label: 'FAKE ON', tone: 'live', ring: 'live', state: 1 }
      : { glyph: 'micOff', label: 'FAKE', state: 0 }),
    on: { fakeMute: true },
  },
  {
    id: 'fakedeafen', name: 'Fake Deafen', tooltip: 'Look deafened to others while you still hear', press: 'fake-deafen',
    look: (s) => (s.fakeDeafen
      ? { glyph: 'headphonesOff', label: 'FAKE ON', tone: 'live', ring: 'live', state: 1 }
      : { glyph: 'headphonesOff', label: 'FAKE', state: 0 }),
    on: { fakeDeafen: true },
  },
];

/** A key while HaloCord can't be reached: its resting picture, dimmed, with a red dot. */
function unreachable(action) {
  const look = action.look({ ...RESTING, ...(action.off || {}) });
  return { ...look, tone: look.tone === 'faint' ? 'faint' : 'dim', ring: null, offline: true, state: 0 };
}

/** The two pictures the manifest lists for a key: state 0, then state 1. */
function manifestLooks(action) {
  const first = action.look({ ...RESTING, ...(action.off || {}) });
  const second = action.look({ ...RESTING, ...(action.on || {}) });
  return [first, second];
}

module.exports = { ACTIONS, RESTING, unreachable, manifestLooks };
