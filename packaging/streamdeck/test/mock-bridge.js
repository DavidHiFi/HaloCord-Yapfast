'use strict';
// A stand-in for HaloCord's Stream Deck server, with the same door rules
// (GET only, the deck header, a local Host, no Origin), for testing the
// Stream Deck plugin before the HaloCord build with the real one ships.
// Every request is logged with its headers. Run: node test/mock-bridge.js <log>

const http = require('node:http');
const fs = require('node:fs');

const logFile = process.argv[2] || 'mock-bridge.log';
const state = {
  ok: true, inCall: true, ready: true, mute: false, deafen: false, camera: false, canFilm: true,
  sharing: true, canShare: true, fakeMute: true, fakeDeafen: false, pttMode: false, talking: false,
  muteHeld: false,
  screens: [
    { slot: 1, present: true, active: false },
    { slot: 2, present: true, active: true },
    { slot: 3, present: true, active: false },
    { slot: 4, present: false, active: false },
  ],
};

function log(line) {
  fs.appendFileSync(logFile, `${new Date().toISOString()} ${line}\n`);
}

http.createServer((request, response) => {
  const headers = request.headers;
  const host = headers.host;
  let status = 200;
  let body;
  if (request.method !== 'GET') { status = 405; body = { ok: false, error: 'only GET' }; }
  else if (headers['x-halocord-deck'] !== '1' || headers.origin || !['127.0.0.1:47830', 'localhost:47830'].includes(host)) {
    status = 403; body = { ok: false, error: 'not the deck' };
  } else if (request.url === '/state') {
    body = state;
  } else if (request.url.startsWith('/press/')) {
    const what = request.url.slice(7);
    const flip = { mute: 'mute', deafen: 'deafen', camera: 'camera', 'fake-mute': 'fakeMute', 'fake-deafen': 'fakeDeafen', 'voice-mode': 'pttMode' }[what];
    if (flip) state[flip] = !state[flip];
    if (what === 'talk-down') state.talking = true;
    if (what === 'talk-up') state.talking = false;
    if (what === 'mute-hold-down') state.muteHeld = true;
    if (what === 'mute-hold-up') state.muteHeld = false;
    body = what === 'screen-4' ? { ok: false, error: 'No screen there' } : { ok: true };
  } else {
    status = 404; body = { ok: false, error: 'no such key' };
  }
  log(`${request.method} ${request.url} -> ${status} headers=${JSON.stringify(headers)}`);
  const text = JSON.stringify(body);
  response.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store', Connection: 'close', 'Content-Length': Buffer.byteLength(text) });
  response.end(text);
}).listen(47830, '127.0.0.1', () => log('listening'));

setTimeout(() => process.exit(0), 10 * 60 * 1000);
