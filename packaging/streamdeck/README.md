# HaloCord keys for the Stream Deck

The Stream Deck plugin `com.davidhifi.yapfast` gives HaloCord for Yapfast its own
keys: mute, deafen, camera, screen share, one key per screen (left, main, right,
top), voice mode, push to talk, push to mute, leave call, fake mute and fake
deafen. It talks to HaloCord's Stream Deck plugin on `127.0.0.1:47830` (see
`docs/halocord.md`), so turn that plugin on in HaloCord first.

It's JavaScript because Stream Deck runs every plugin in its own Node.

## Files

- `com.davidhifi.yapfast.sdPlugin/`: the plugin as Stream Deck loads it.
  `bin/plugin.js` talks to Stream Deck and HaloCord, `bin/actions.js` says what
  each key does and shows, `bin/icons.js` draws the keys (Catppuccin Mocha,
  blue to mauve when live, red when muted or deafened, a red dot when HaloCord
  can't be reached).
- `build.js`: writes `manifest.json` and the default key pictures from
  `bin/actions.js`. Run it after changing a key.
- `plugin_icon.py`: draws `imgs/plugin.png` and `plugin@2x.png` (needs Pillow).
- `preview.py`: draws every key in every look onto one sheet with Qt's SVG
  renderer, the same one Stream Deck uses (needs PySide6).
- `repoint_keys.py`: moves existing keys from Elgato's Discord plugin to these
  ones (`check`, then `apply` with Stream Deck closed; each page is backed up
  first).
- `test/fake-host.js`: starts the plugin the way Stream Deck does, against
  `test/mock-bridge.js`, shows every key, presses them and checks what comes
  back.

## Build, test, install

```
node build.js
python plugin_icon.py
node test/fake-host.js
```

Install by copying `com.davidhifi.yapfast.sdPlugin` into
`%APPDATA%\Elgato\StreamDeck\Plugins\` while Stream Deck is closed, then start
Stream Deck. The keys show under HaloCord in the action list.
