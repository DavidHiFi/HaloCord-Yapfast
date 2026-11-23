'use strict';
// HaloCord for Yapfast's Stream Deck keys. Stream Deck starts this with its
// own Node; it talks to Stream Deck over a websocket and to HaloCord's
// Stream Deck plugin over plain HTTP on this computer (127.0.0.1:47830).
// Every key reads one shared poll of HaloCord's state.

const http = require('node:http');
const { ACTIONS, unreachable } = require('./actions.js');
const { key, dataUri } = require('./icons.js');

const BRIDGE = { host: '127.0.0.1', port: 47830 };
const POLL_EVERY = 1500;
const HOLD_REPEAT = 1000;
const ANSWER_WITHIN = 1500;
const PREFIX = 'com.davidhifi.yapfast.';

const byId = new Map(ACTIONS.map((action) => [action.id, action]));

/** Stream Deck's arguments: -port, -pluginUUID, -registerEvent, -info. */
const args = {};
for (let i = 2; i + 1 < process.argv.length; i += 2) {
  args[process.argv[i].replace(/^-+/, '')] = process.argv[i + 1];
}

/** context -> { action, picture, state, repeat } */
const keys = new Map();
let socket = null;
let latest = null;
let polling = false;
let timer = null;
let lastError = '';

function log(message) {
  console.log(new Date().toISOString(), message);
  send({ event: 'logMessage', payload: { message: `HaloCord deck: ${message}` } });
}

function send(message) {
  if (socket && socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message));
}

/** One request to HaloCord; null when it can't be reached. */
function ask(path) {
  return new Promise((resolve) => {
    const request = http.get(
      { ...BRIDGE, path, agent: false, headers: { 'X-HaloCord-Deck': '1' }, timeout: ANSWER_WITHIN },
      (response) => {
        let body = '';
        response.setEncoding('utf8');
        response.on('data', (part) => {
          body += part;
          if (body.length > 65536) request.destroy();
        });
        response.on('end', () => {
          try {
            resolve(JSON.parse(body));
          } catch {
            resolve(null);
          }
        });
      },
    );
    request.on('timeout', () => request.destroy());
    request.on('error', () => resolve(null));
  });
}

function draw(context, entry) {
  const look = latest && latest.ok ? entry.action.look(latest) : unreachable(entry.action);
  if (latest && latest.ok && !latest.ready) look.tone = look.tone === 'faint' ? 'faint' : 'dim';
  const picture = key(look);
  if (look.state !== entry.state) {
    entry.state = look.state;
    send({ event: 'setState', context, payload: { state: look.state } });
  }
  if (picture !== entry.picture) {
    entry.picture = picture;
    send({ event: 'setImage', context, payload: { image: dataUri(picture), target: 0 } });
  }
}

function drawAll() {
  for (const [context, entry] of keys) draw(context, entry);
}

async function poll() {
  if (polling) return;
  polling = true;
  const state = await ask('/state');
  polling = false;
  const why = !state ? 'HaloCord is not running, or its Stream Deck plugin is off' : state.ok ? '' : state.error || 'no answer';
  if (why !== lastError) {
    lastError = why;
    log(why ? `can't read HaloCord: ${why}` : 'connected to HaloCord');
  }
  latest = state;
  drawAll();
}

function keepPolling() {
  if (keys.size > 0 && !timer) {
    timer = setInterval(poll, POLL_EVERY);
    poll();
  } else if (keys.size === 0 && timer) {
    clearInterval(timer);
    timer = null;
  }
}

async function press(context, path) {
  const answer = await ask(path);
  if (!answer || !answer.ok) {
    send({ event: 'showAlert', context });
    log(`${path}: ${answer ? answer.error : 'HaloCord is not running'}`);
  }
  poll();
}

function letGo(context, entry) {
  if (!entry.repeat) return;
  clearInterval(entry.repeat);
  entry.repeat = null;
  press(context, `/press/${entry.action.hold}-up`);
}

function received(message) {
  const { event, action: uuid, context } = message;
  const id = typeof uuid === 'string' && uuid.startsWith(PREFIX) ? uuid.slice(PREFIX.length) : null;
  const action = id ? byId.get(id) : null;
  switch (event) {
    case 'willAppear':
      if (!action) return;
      keys.set(context, { action, picture: '', state: -1, repeat: null });
      draw(context, keys.get(context));
      keepPolling();
      return;
    case 'willDisappear': {
      const entry = keys.get(context);
      if (entry) letGo(context, entry);
      keys.delete(context);
      keepPolling();
      return;
    }
    case 'keyDown': {
      const entry = keys.get(context);
      if (!entry) return;
      if (entry.action.hold) {
        if (entry.repeat) return;
        const down = `/press/${entry.action.hold}-down`;
        entry.repeat = setInterval(() => ask(down), HOLD_REPEAT);
        press(context, down);
      } else {
        press(context, `/press/${entry.action.press}`);
      }
      return;
    }
    case 'keyUp': {
      const entry = keys.get(context);
      if (entry && entry.action.hold) letGo(context, entry);
      else if (entry) draw(context, entry);
      return;
    }
    case 'systemDidWakeUp':
      poll();
      return;
    default:
  }
}

function connect() {
  socket = new WebSocket(`ws://127.0.0.1:${args.port}`);
  socket.addEventListener('open', () => {
    socket.send(JSON.stringify({ event: args.registerEvent, uuid: args.pluginUUID }));
    log('started');
  });
  socket.addEventListener('message', (event) => {
    try {
      received(JSON.parse(event.data));
    } catch (error) {
      console.error(error);
    }
  });
  socket.addEventListener('close', () => process.exit(0));
}

if (args.port && args.pluginUUID && args.registerEvent) {
  connect();
} else {
  module.exports = { ask, ACTIONS };
}
