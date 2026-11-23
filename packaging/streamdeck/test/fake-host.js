'use strict';
// A stand-in for the Stream Deck app: starts the plugin the way Stream Deck
// does, shows it every key, presses some, and checks what the plugin sends
// back and what reaches the stand-in HaloCord bridge (mock-bridge.js).
// Run with Node 24 or newer (Stream Deck's own works): node test/fake-host.js

const http = require('node:http');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { spawn } = require('node:child_process');

const HERE = __dirname;
const PLUGIN = path.join(HERE, '..', 'com.davidhifi.yapfast.sdPlugin');
const MOCK_LOG = path.join(require('node:os').tmpdir(), 'halocord-deck-test-mock.log');
const IDS = ['mute', 'deafen', 'camera', 'share', 'voicemode', 'talk', 'mutehold', 'leave',
  'screen1', 'screen2', 'screen3', 'screen4', 'fakemute', 'fakedeafen'];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const got = []; // every message from the plugin
let client = null;
let mock = null;

function frame(text) {
  const data = Buffer.from(text);
  const head = data.length < 126 ? Buffer.from([0x81, data.length])
    : Buffer.from([0x81, 126, data.length >> 8, data.length & 255]);
  return Buffer.concat([head, data]);
}

function sendEvent(message) { client.write(frame(JSON.stringify(message))); }

function readFrames(socket) {
  let buf = Buffer.alloc(0);
  socket.on('data', (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    for (;;) {
      if (buf.length < 2) return;
      const op = buf[0] & 15;
      let len = buf[1] & 127;
      let at = 2;
      if (len === 126) { if (buf.length < 4) return; len = buf.readUInt16BE(2); at = 4; }
      else if (len === 127) { if (buf.length < 10) return; len = Number(buf.readBigUInt64BE(2)); at = 10; }
      const masked = buf[1] & 128;
      const need = at + (masked ? 4 : 0) + len;
      if (buf.length < need) return;
      const mask = masked ? buf.subarray(at, at + 4) : null;
      const body = Buffer.from(buf.subarray(at + (masked ? 4 : 0), need));
      if (mask) for (let i = 0; i < body.length; i++) body[i] ^= mask[i & 3];
      buf = buf.subarray(need);
      if (op === 1) got.push({ at: Date.now(), ...JSON.parse(body.toString()) });
    }
  });
}

function startMock() {
  mock = spawn(process.execPath, [path.join(HERE, 'mock-bridge.js'), MOCK_LOG], { windowsHide: true, stdio: 'ignore' });
  return sleep(600);
}

function mockLines() {
  return fs.existsSync(MOCK_LOG) ? fs.readFileSync(MOCK_LOG, 'utf8').split('\n').filter(Boolean) : [];
}

function lastImage(context) {
  const hit = [...got].reverse().find((m) => m.event === 'setImage' && m.context === context);
  if (!hit) return null;
  return Buffer.from(hit.payload.image.split(',')[1], 'base64').toString('utf8');
}

const labelOf = (svg) => (svg && /<text[^>]*font-size="19"[^>]*>([^<]*)</.exec(svg) || [])[1] || null;
const offline = (svg) => !!svg && svg.includes('r="9"');

async function main() {
  try { fs.unlinkSync(MOCK_LOG); } catch {}
  await startMock();
  const server = http.createServer();
  server.on('upgrade', (req, socket) => {
    const accept = crypto.createHash('sha1').update(req.headers['sec-websocket-key'] + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    client = socket;
    readFrames(socket);
  });
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const port = server.address().port;
  const plugin = spawn(process.execPath, ['bin/plugin.js', '-port', String(port), '-pluginUUID', 'test-uuid',
    '-registerEvent', 'registerPlugin', '-info', '{}'], { cwd: PLUGIN, windowsHide: true, stdio: 'ignore' });

  const results = {};
  const check = (name, ok, detail) => { results[name] = { ok: !!ok, detail }; };
  for (let i = 0; i < 50 && !got.some((m) => m.event === 'registerPlugin'); i++) await sleep(100);
  check('registers', got.some((m) => m.event === 'registerPlugin' && m.uuid === 'test-uuid'));

  for (const id of IDS) sendEvent({ event: 'willAppear', action: `com.davidhifi.yapfast.${id}`, context: id, device: 'd', payload: { settings: {}, coordinates: { column: 0, row: 0 } } });
  await sleep(2500);
  const labels = Object.fromEntries(IDS.map((id) => [id, labelOf(lastImage(id))]));
  check('every key drawn', IDS.every((id) => labels[id]), labels);
  check('polls state', mockLines().some((l) => l.includes('GET /state -> 200')));

  sendEvent({ event: 'keyDown', action: 'com.davidhifi.yapfast.mute', context: 'mute', payload: {} });
  sendEvent({ event: 'keyUp', action: 'com.davidhifi.yapfast.mute', context: 'mute', payload: {} });
  await sleep(900);
  check('mute press reaches HaloCord', mockLines().some((l) => l.includes('GET /press/mute ')));
  check('mute key turns red', labelOf(lastImage('mute')) === 'MUTED', labelOf(lastImage('mute')));

  sendEvent({ event: 'keyDown', action: 'com.davidhifi.yapfast.talk', context: 'talk', payload: {} });
  await sleep(2600);
  const talkingLabel = labelOf(lastImage('talk'));
  sendEvent({ event: 'keyUp', action: 'com.davidhifi.yapfast.talk', context: 'talk', payload: {} });
  await sleep(900);
  const downs = mockLines().filter((l) => l.includes('/press/talk-down')).length;
  check('hold talk repeats while held', downs >= 3, `${downs} downs, label while held ${talkingLabel}`);
  check('talk lets go on release', mockLines().some((l) => l.includes('/press/talk-up')), labelOf(lastImage('talk')));

  for (const id of ['screen1', 'fakemute', 'fakedeafen', 'share']) {
    sendEvent({ event: 'keyDown', action: `com.davidhifi.yapfast.${id}`, context: id, payload: {} });
    sendEvent({ event: 'keyUp', action: `com.davidhifi.yapfast.${id}`, context: id, payload: {} });
  }
  await sleep(900);
  check('screen 1 press', mockLines().some((l) => l.includes('/press/screen-1 ')));
  check('fake mute flips', labelOf(lastImage('fakemute')) === 'FAKE', labelOf(lastImage('fakemute')));
  check('fake deafen flips', labelOf(lastImage('fakedeafen')) === 'FAKE ON', labelOf(lastImage('fakedeafen')));

  const alertsBefore = got.filter((m) => m.event === 'showAlert' && m.context === 'screen4').length;
  sendEvent({ event: 'keyDown', action: 'com.davidhifi.yapfast.screen4', context: 'screen4', payload: {} });
  sendEvent({ event: 'keyUp', action: 'com.davidhifi.yapfast.screen4', context: 'screen4', payload: {} });
  await sleep(900);
  check('missing screen shows the warning', got.filter((m) => m.event === 'showAlert' && m.context === 'screen4').length > alertsBefore);

  mock.kill();
  await sleep(3500);
  check('HaloCord gone dims every key', IDS.every((id) => offline(lastImage(id))));
  await startMock();
  await sleep(3500);
  check('comes back by itself', IDS.every((id) => !offline(lastImage(id))));

  for (const id of IDS) sendEvent({ event: 'willDisappear', action: `com.davidhifi.yapfast.${id}`, context: id, payload: {} });
  await sleep(500);
  const before = mockLines().filter((l) => l.includes('/state')).length;
  await sleep(4000);
  const after = mockLines().filter((l) => l.includes('/state')).length;
  check('stops asking when no key shows', after === before, `${after - before} asks after hiding`);

  const headers = mockLines().filter((l) => l.includes('headers=')).map((l) => JSON.parse(l.split('headers=')[1]));
  check('right door headers, no origin', headers.length > 0 && headers.every((h) => h['x-halocord-deck'] === '1' && h.host === '127.0.0.1:47830' && !h.origin));

  plugin.kill();
  mock.kill();
  server.close();
  client.destroy();
  const passed = Object.values(results).filter((r) => r.ok).length;
  console.log(JSON.stringify({ passed, total: Object.keys(results).length, results }, null, 1));
  process.exit(passed === Object.keys(results).length ? 0 : 1);
}

main();
