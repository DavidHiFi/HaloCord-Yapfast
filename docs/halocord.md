# HaloCord's build of Yapfast

HaloCord's copy of Yapfast keeps its own code in its own files, so the updater can
replay it onto each Yapfast release with little to conflict. Yapfast's own files
carry only the hooks listed here. When a rebase conflicts, these lines are the
whole list to carry across.

## HaloCord's own files

- `crates/yapfast-iced/src/halocord.rs`: the plugin list (`PLUGINS`), each plugin's
  settings (a choice, a switch, a slider or a text), what `layout.json` keeps
  (`halocord.plugins.<id>`), what each plugin tells the core, and the hooks it plugs
  in while it's on. Adding a plugin is one entry here. Also the window's side of
  plug-in point 5 (`menu_rows`, `pick`).
- `crates/yapfast-iced/src/settings/pages/halocord.rs`: the HaloCord settings page,
  drawn from `PLUGINS`.
- `crates/yapfast-core/src/halocord.rs`: switches the core reads (flat microphone,
  bit rate), and the plug-in points: the `Hooks` a plugin writes, `plug` to put
  them in or take them out, and the calls each hook below makes.
- `crates/yapfast-core/src/discord_audio/flat.rs`: the stereo loader's flat
  microphone, and the switch between it and Yapfast's own capture mid-call.

## Plug-in points

A plugin writes only the `Hooks` it needs; each does nothing unless written. The
numbers match the port plan.

1. Actions and events, before and after (`application.rs`): `dispatch` and `apply`
   ask `keeps_action` / `keeps_event` first and return nothing if a plugin drops it;
   both end with `halocord::after`, which may change or add to the effects. An event
   that opens a room runs a nested `dispatch`, so `after` can see both.
2. Commands on their way out (`yapfast-iced/src/client.rs`, `execute`): a
   `Backend` effect a plugin holds back (`halocord::sends`) isn't sent.
3. Live Discord events (`discord_text/commands.rs`, `live`): `halocord::live` runs
   before the interactions and the channels see the event; a dropped one goes
   nowhere.
4. Message extras (`yapfast-iced/src/message/mod.rs`, `author_line`): the plugins'
   tags after OP or APP, in the same chip.
5. Menu rows (`yapfast-iced/src/menus.rs`): `MenuAct::Plugin`; `open_menu` adds
   `halocord::menu_rows` before Delete; `carry_out` sends the row's key
   (`plugin-id/action`) to `halocord::pick`; `act` lists it with the menus' own.
   A row's pick gives actions for the app and text to copy.
6. Setting kinds: in HaloCord's own files only. Kept values stay plain JSON (a
   number, `true`/`false` or a string), so the stereo loader's saved bit rate reads
   as before.

## Hooks in Yapfast's files

Window (`yapfast-iced`):

- `lib.rs`: `mod halocord;`.
- `layout.rs`: the `halocord: crate::halocord::Kept` field on `Layout`.
- `app/mod.rs`: `layout.halocord.apply();` after `layout.voice.apply();` at start.
- `settings/mod.rs`: `Section::HaloCord`, its title and icon, its group, and its page.
- `settings/pages/mod.rs`: `mod halocord;`, `Pref::HaloCord(Change)`, and its arm in
  the change handler, which keeps it under `halocord`.
- `client.rs`: plug-in point 2, one match arm in `execute`.
- `message/mod.rs`: plug-in point 4, a loop in `author_line`.
- `menus.rs`: plug-in point 5, `MenuAct::Plugin` and its three lines.
- `tests/settings.rs`: `halocord_sends_stereo_at_510_unless_changed`.

Core (`yapfast-core`):

- `lib.rs`: `pub mod halocord;`.
- `application.rs`: plug-in point 1, two checks and two `after` calls.
- `discord_text/commands.rs`: plug-in point 3, `halocord::live` in `live`.
- `discord_audio.rs`:
  - `mod flat;`.
  - `Gate` takes its block size as a const generic (`Gate<const N: usize = BLOCK>`),
    so the flat microphone gates stereo blocks with the same gate.
  - `MicrophoneThread::start` calls `flat::run(request, denoiser)` in place of
    `request.run(denoiser)`, and `CaptureRequest::run` takes `&self`.
  - `GateMeter` calls `flat::measure(...)` in place of `measure(...)`.
  - The loops in `CaptureRequest::run` and `measure` also end on `flat::on()`, so
    the flat path takes over within a block.

Calls (`yapfast-discord`):

- `voice.rs`: the microphone bit rate as a setting (`set_microphone_bitrate`,
  `microphone_bitrate`, 128 kbit/s unless changed, clamped to what Opus sends),
  and `Microphone` keeps interleaved stereo (`push` writes mono to both channels,
  `push_stereo` writes a pair as it came).
- `call_connection.rs`: the encoder reads the bit rate and changes it mid-call;
  `take_frame` takes interleaved stereo as it is.
- `opus.rs`: a test that the bit rate changes within what Opus takes.

## Tests

- Core `halocord::tests`: points 1 to 5 through a test plugin (each test thread
  has its own plugged list, so they don't meet).
- Window `halocord::tests`: point 5's rows, point 6's kinds, the stereo loader's
  saved values.

## Candidates to offer upstream

- The microphone bit rate setting and stereo `Microphone` in `yapfast-discord`.
- The const-generic `Gate`.
- Push to talk's say for a block (copied into `flat.rs` from `Processing::pair`).
- The plug-in points, as a plugin layer for Yapfast.
