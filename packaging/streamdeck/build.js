'use strict';
// Writes the plugin's manifest and its key pictures from bin/actions.js and
// bin/icons.js, so the pictures Stream Deck shows before the plugin starts
// match the ones it draws. Run: node build.js

const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, 'com.davidhifi.yapfast.sdPlugin');
const { ACTIONS, manifestLooks } = require(path.join(root, 'bin', 'actions.js'));
const { key, listIcon } = require(path.join(root, 'bin', 'icons.js'));

const VERSION = '1.0.0.0';

function write(relative, text) {
  const file = path.join(root, relative);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text, 'utf8');
}

const actions = ACTIONS.map((action) => {
  const looks = manifestLooks(action);
  write(`imgs/actions/${action.id}.svg`, listIcon(looks[0].glyph, looks[0].digit));
  looks.forEach((look, state) => write(`imgs/keys/${action.id}-${state}.svg`, key(look)));
  return {
    Name: action.name,
    UUID: `com.davidhifi.yapfast.${action.id}`,
    Icon: `imgs/actions/${action.id}`,
    Tooltip: action.tooltip,
    Controllers: ['Keypad'],
    DisableAutomaticStates: true,
    UserTitleEnabled: false,
    SupportedInMultiActions: !action.hold,
    States: looks.map((_, state) => ({ Image: `imgs/keys/${action.id}-${state}`, ShowTitle: false })),
  };
});

write('imgs/category.svg', listIcon('voiceOpen'));

const manifest = {
  $schema: 'https://schemas.elgato.com/streamdeck/plugins/manifest.json',
  SDKVersion: 2,
  UUID: 'com.davidhifi.yapfast',
  Author: 'DavidHiFi',
  Name: 'HaloCord for Yapfast',
  Description: 'Mute, deafen, camera, screen share with one key per screen, push to talk and the fake voice switches for HaloCord for Yapfast. Needs the Stream Deck plugin turned on in HaloCord.',
  Version: VERSION,
  Icon: 'imgs/plugin',
  URL: 'https://github.com/DavidHiFi/HaloCord-Yapfast',
  CodePath: 'bin/plugin.js',
  Nodejs: { Version: '24', Debug: 'disabled' },
  Category: 'HaloCord',
  CategoryIcon: 'imgs/category',
  OS: [{ Platform: 'windows', MinimumVersion: '10' }],
  Software: { MinimumVersion: '6.5' },
  Actions: actions,
};
write('manifest.json', `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`wrote ${actions.length} actions`);
