# HaloCord's build of Yapfast

HaloCord's copy of Yapfast keeps its own code in its own files, so the updater can
replay it onto each Yapfast release with little to conflict. Yapfast's own files
carry only the hooks listed here. When a rebase conflicts, these lines are the
whole list to carry across.

## HaloCord's own files

- `crates/yapfast-iced/src/halocord/`: the window's side of the plugins, one file
  per area, matching the core's `halocord/`.
  - `mod.rs`: the plugin ids (`id`), the plugin list (`PLUGINS`, which only names
    each plugin in the page's order), and search.
  - `kept.rs`: what a plugin is, its settings (a choice, a switch, a slider or a
    text), what `layout.json` keeps (`halocord.plugins.<id>`) and `changed`.
  - `voice.rs`, `chat.rs`, `menus.rs`, `buttons.rs`, `members.rs`: the plugins of
    each area, what each tells the core, the hooks it plugs in while it's on, and
    what the window draws for it. `menus.rs` is the window's side of plug-in
    point 5 (`menu_rows`, `pick`).
- `crates/yapfast-iced/src/settings/pages/halocord.rs`: the HaloCord settings page,
  drawn from `PLUGINS`.
- `crates/yapfast-iced/src/mochacord.rs`: MochaCord's look: what `layout.json` keeps
  under `mochacord` (glass on and strength, accent gradient, corner roundness, font
  and weight, and the one-time switch flag), applying it, the glass fills' alpha
  (`glass`, `Dock`), the first-start switch to the theme, and the Windows backdrop
  (direct `dwmapi` calls, so no new dependency).
- `crates/yapfast-iced/src/settings/pages/mochacord.rs`: the MochaCord settings page.
- `crates/yapfast-iced-kit/src/fonts/pick.rs`: gives the chosen font Yapfast's text
  name at each weight, and keeps Yapfast's own font behind it.
- `crates/yapfast-iced-kit/icons/halo-logo.svg`, `packaging/icon.svg`,
  `packaging/windows/yapfast.ico` and `scripts/make_windows_icon.py`: the blue to
  mauve icon (`scripts/make-windows-icon` runs the script).
- `crates/yapfast-core/src/halocord.rs`: switches the core reads (flat microphone,
  bit rate, fake mute and fake deafen: `voice_state` is what others are sent), `Tag`,
  and the plug-in points: the `Hooks` a plugin writes, `plug` to put
  them in or take them out, and the calls each hook below makes.
- `crates/yapfast-core/src/halocord/staff.rs`: the staff tag plugin's hook (owner,
  admin and mod by names in chat) and its crown and role-colour switches.
- `crates/yapfast-core/src/halocord/copy_user_mention.rs`: Copy mention in a
  Discord user's menu.
- `crates/yapfast-core/src/halocord/copy_media_link.rs`: Copy file link in a file's
  menu.
- `crates/yapfast-core/src/halocord/copy_file_contents.rs`: Copy file text in a text
  file's menu (downloads it as Copy Image does; the window copies it as text).
- `crates/yapfast-discord/src/halocord.rs`: Platform indicators' record of which of
  Discord's apps each person is on, from a presence's `client_status`.
- `crates/yapfast-iced-kit/icons/halo-desktop.svg`, `halo-phone.svg`, `halo-web.svg`:
  its three marks.
- `crates/yapfast-core/src/halocord/copy_sticker_link.rs`: Copy sticker link in a
  sticker's menu.
- `crates/yapfast-core/src/halocord/quick_mention.rs`: Mention in a message's menu
  (puts the sender's name in the message box).
- `crates/yapfast-core/src/halocord/copy_user_url.rs`: Copy profile link in a
  Discord user's menu.
- `crates/yapfast-core/src/halocord/new_user.rs`: the new account tag plugin's hook (an amber tag by
  accounts younger than N days, read from the Discord id) and its day count.
- `crates/yapfast-core/src/discord_audio/flat.rs`: the stereo loader's flat
  microphone, and the switch between it and Yapfast's own capture mid-call.

## Adding a plugin

1. If it needs the core (a hook on actions, events, commands, menus or tags), add
   `crates/yapfast-core/src/halocord/<name>.rs` with a unit struct that implements
   `Hooks` (its `id` is the plugin's id), and its tests. Name it in `halocord.rs`
   (`pub mod <name>;`).
2. Add its id to `id` in `crates/yapfast-iced/src/halocord/mod.rs`. The id is what
   `layout.json` keeps, so it never changes once shipped.
3. Add one `const` in its area's file:

   ```rust
   pub(super) const MY_PLUGIN: Plugin = Plugin {
       id: id::MY_PLUGIN,
       name: "My plugin",
       about: "What it does, in one plain sentence.",
       on_at_first: false,
       settings: &[off("my-switch", "A switch", "What the switch does.")],
       apply: |_, _| {},
       hooks: Some(&engine::my_plugin::MyPlugin),
   };
   ```

4. Put it in `PLUGINS` where it belongs on the page.
5. Tests: one for what the hook does (core), and one that switches it on through
   `changed` and checks the window (`tests/halocord_buttons.rs` has examples).
6. If a Yapfast file has to call in, add the call and name the file under "Hooks in
   Yapfast's files". The window test `every_hook_is_in_the_docs` fails otherwise.
7. `scripts/halocord-check.ps1` must pass before a release.

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
   tags after OP or APP. `extras(message, app)` gives `Tag`s (words, crown, colour);
   `halocord::tag` draws one, in the APP chip when it has no colour.
5. Menu rows (`yapfast-iced/src/menus.rs`): `MenuAct::Plugin`; `open_menu` adds
   `halocord::menu_rows` before Delete; `carry_out` sends the row's key
   (`plugin-id/action`) to `halocord::pick`; `act` lists it with the menus' own.
   A row's pick gives actions for the app and text to copy. A person's menu takes
   rows the same way (`person_menu`, `person_pick`): `open_person_menu` adds
   `halocord::person_rows`, and `carry_out` sends a row picked in a menu with a
   `place` to `halocord::chosen`, which routes it to the person.
   A file's or sticker's menu takes rows too (`attachment_menu`, `attachment_pick`):
   a right click sends `image_viewer::Event::Attachment`, `halocord::attachment_menu`
   opens the rows, and a key ending `#<index>` goes to `halocord::attachment_pick`.
6. Setting kinds: in HaloCord's own files only. Kept values stay plain JSON (a
   number, `true`/`false` or a string), so the stereo loader's saved bit rate reads
   as before.

## Hooks in Yapfast's files

Window (`yapfast-iced`):

- `lib.rs`: `mod halocord;` and `mod mochacord;`; in `run`, `mochacord::prepare(&root);`
  before `fonts::install`.
- `layout.rs`: the `mochacord: crate::mochacord::Kept` field on `Layout`.
- `app/mod.rs`: `mochacord::first_start` after the layout is opened, and
  `layout.mochacord.apply();` after `layout.halocord.apply();`.
- `app/view.rs`: `mochacord::glass(..)` on the navigation's fill and on the window's
  clear colour (`window_style`).
- `sidebar/mod.rs`: the same on the channel list's ground. `window_bar.rs`: the same on
  the title bar's ground.
- `shell.rs`: `transparent` in `window_settings`; `mochacord::frame(window)` in
  `opened`; `mochacord::set_in_front` in `focus_changed`.
- `rail.rs`: `HOME_ROOM` above the home tile, and `logo_tile` (the riced icon).
- `settings/mod.rs`: `Section::MochaCord`, its title, icon, group entry and page.
- `settings/pages/mod.rs`: `mod mochacord;`, `Pref::MochaCord(Change)` and its arm.
- `layout.rs`: the `halocord: crate::halocord::Kept` field on `Layout`.
- `app/mod.rs`: `layout.halocord.apply();` before `layout.voice.apply();` at start (the
  Volume booster's cap is set before people's saved volumes play).
- `settings/mod.rs`: `Section::HaloCord`, its title ("Plugins") and icon, its group,
  and its page; Settings' own search keeps the section when `halocord::any_found`
  finds a plugin (the page narrows its list the same way).
- `settings/pages/mod.rs`: `mod halocord;`, `Pref::HaloCord(Change)`, and its arm in
  the change handler, which calls `halocord::changed` (keeps it under `halocord`,
  and sends the voice state again when a fake mute or deafen switch changes in a call).
- `client.rs`: plug-in point 2, one match arm in `execute`.
- `message/mod.rs`: plug-in point 4, a loop in `author_line`.
- `menus.rs`: plug-in point 5, `MenuAct::Plugin` and its three lines.
- `person_menu.rs`: plug-in point 5, `open_person_menu` adds `halocord::person_rows`
  (two lines, `rows` is `mut`).
- `message/attachments.rs`: plug-in point 5, `file_card` and `stickers` wrap what they draw in a
  right click (`RightPress`, `Event::Attachment`; a few lines each).
- `image_viewer.rs`: the `Event::Attachment` variant and its empty arm.
- `app/screens.rs`: one arm in `image_event` calling `halocord::attachment_menu`.
- `account.rs`: the User area plugin's options. `height(compact)` beside `HEIGHT`,
  `Event::Plugin(Change)`, and `view` reads `layout.halocord.dock()` for compact mode
  (smaller, inside the channel list), hidden dropdown arrows and the extra buttons,
  and says "Faking mute" or "Faking deafen" under the name in a call.
- `status.rs`: the popover's bottom uses `account::height(compact)`.
- `call/popups/person.rs`: `volumes_view` ends the sliders at `halocord::volume_cap()`
  (200 unless the Volume booster is on), in place of 200.
- `call/mod.rs`: `Event::Unmute(person)` and its arm, which calls `halocord::unmute`.
- `sidebar/voice.rs`: `voice_person` ends its marks with `halocord::unmute_mark`, a click-to-unmute
  mark beside someone muted for this account (one `if let`).
- `sidebar/channels.rs`: `room_row` pushes `halocord::voice_count` (the Voice count plugin's number)
  before the badge on a voice channel's row (one `if let`).
- `members.rs`: `member_row` pushes `halocord::in_voice_mark` after the timed-out clock, then
  `halocord::platform_marks` (one `if let` each).
- `shell.rs`: in `shell_sync`, a copied download goes through `halocord::copied_text` first:
  text Copy file text asked for goes on the clipboard as text, anything else as a picture.
- `tests/settings.rs`: `halocord_sends_stereo_at_510_unless_changed`.
- `tests/calls.rs`, `tests/media.rs`, `tests/menus.rs`, `tests/navigation.rs`: tests of
  the Volume booster, the copy rows and the buttons beside Settings, in Yapfast's own
  test files.
- `window_bar.rs`: `Event::Halo(Press)` with its arm in `update` (a plugin's button
  pressed), and the buttons `halocord::presses` lists, pushed before Inbox in
  `window_bar`.
- `account.rs`: the same buttons beside Settings, when the window has no title bar of
  its own.
- `composer.rs`: `composer_box` pushes one slot (the counter, or a `Space`) between the field and the emoji button.
- `sidebar/channels.rs`: the Direct Messages heading pushes the close all button when its plugin is on.
- `sync.rs`: after the member list's lines are made, `halocord::add_member_count` puts the count's line first.
- `tests/mod.rs`: `mod halocord_buttons;`, `mod halocord_button_pictures;` and
  `mod halocord_fixes;`.

Kit (`yapfast-iced-kit`) and vendor:

- `fonts.rs`: `mod pick;`, and a call in `install` before the fallback list.
- `style.rs`: `set_corner_scale`, read in `rounded`, `edge` and `round`; `controls.rs` `lit_style` takes its corners from `round`.
- `widgets/smooth.rs` and `smooth/bar.rs`: `BarGradient` and `set_bar_look`, drawn in
  `Smooth::draw`.
- `renderer.rs`: `want_glass`, `glass_wanted`, `glass_active`.
- `icons.rs`: `Icon::HaloLogo`, `HaloDesktop`, `HaloPhone`, `HaloWeb`.
- `vendor/iced_wgpu`: `glass.rs` and the glass attempt in `window/compositor.rs` (see
  its `README.yapfast.md`).
- Core `theme.rs`: the `MOCHACORD` built-in theme.

Core (`yapfast-core`):

- `lib.rs`: `pub mod halocord;`.
- `application.rs`: plug-in point 1, two checks and two `after` calls.
- `application/members.rs`: `member_standing(place, user)`, whether a member owns the
  server and what it lets them do (the staff tag reads it).
- `volume.rs`: `most()` in place of `MAX` in `set` (`MAX`, or what the Volume booster's cap
  allows), and `from_slider` / `to_slider` carry on in a line past 200.
- `discord_voice.rs`: both `set_self` calls send `halocord::voice_state(muted, deafened)`.
- `discord_text/commands.rs`: plug-in point 3, `halocord::live` in `live`.
- `discord_audio.rs`:
  - In `mix`, while `halocord::mono()` (Force mono), a person's voice goes through
    `halocord::add_mono` and the loop moves on (four lines). Stream and Matrix
    sound are left as they are.
  - `mod flat;`.
  - `Gate` takes its block size as a const generic (`Gate<const N: usize = BLOCK>`),
    so the flat microphone gates stereo blocks with the same gate.
  - `MicrophoneThread::start` calls `flat::run(request, denoiser)` in place of
    `request.run(denoiser)`, and `CaptureRequest::run` takes `&self`.
  - `GateMeter` calls `flat::measure(...)` in place of `measure(...)`.
  - The loops in `CaptureRequest::run` and `measure` also end on `flat::on()`, so
    the flat path takes over within a block.

Discord (`yapfast-discord`):

- `lib.rs`: `pub mod halocord;`.
- `member_list.rs`: `presence` and the member list's rows call `halocord::note` with the
  presence (one line each).
- `service/tracker.rs`: `presence` calls `halocord::note` (one line).

Calls (`yapfast-discord`):

- `voice.rs`: the microphone bit rate as a setting (`set_microphone_bitrate`,
  `microphone_bitrate`, 128 kbit/s unless changed, clamped to what Opus sends),
  and `Microphone` keeps interleaved stereo (`push` writes mono to both channels,
  `push_stereo` writes a pair as it came).
- `call_connection.rs`: the encoder reads the bit rate and changes it mid-call;
  `take_frame` takes interleaved stereo as it is.
- `opus.rs`: a test that the bit rate changes within what Opus takes.

## Fixes to Yapfast's own bugs

Each fix is one commit on `david/halocord`, with a window test in
`tests/halocord_fixes.rs` named for the issue. On a rebase, drop a fix once
Yapfast's own fix for it lands (its test still passes without ours).

- yapfast#287, the view doesn't follow a new message from just past the last one:
  `timeline/mod.rs` `NEAR_END` and `near_end`, used in place of `<= 2.0` in
  `sync.rs` (`timeline_scrolled`), `edit.rs` and `timeline/scroll.rs`
  (`scroll_to_row`); `sync.rs` goes to the end as messages change while
  `at_end` and `from_end > 0.5`.
- yapfast#286, a message in the DM being read makes the server bar jump:
  `sync.rs` `make_lists` leaves the selected DM off the rail while `reads()`.
- yapfast#263, text under the hover bar can't be selected:
  `message/hover_bar.rs` `hover_bar` gives no bar while a drag selects
  (unless its menu or picker is open).
- Yapfast's own test, not the app: `tests/timeline.rs`
  `the_history_edge_offers_earlier_messages_then_shows_them_loading` took an
  hour off a preview message's time of 0 and overflowed; it saturates now.
- The GitHub check installs the newest LLVM (`.github/workflows/halocord.yml`):
  the runner's own clang-cl is older than its Visual Studio's C++ library.
- A row of choices too narrow for its options' words (the Stereo loader's five bit
  rates in the Plugins page's 360 px, Messages' date formats, each about 2 px short)
  ran its options into one another:
  `yapfast-iced-kit/src/controls.rs` `choices_within` measures each option's words
  (`choice_words`) and, when they don't all fit, lays them out on as few even lines
  as hold them (`choice_lines`); a row that fits is drawn as before. It lays out
  from `widest`, so a page narrower than that still squeezes it. Test:
  `segmented_rows_wrap_only_when_they_dont_fit`.
- Checked, nothing to change: yapfast#257 (code blocks drawn wrong) came from
  the old Slint window; the iced window draws them right, and the ignored test
  `code_block_picture` in `tests/halocord_fixes.rs` saves a picture to check.

## Tests

- Window `tests/halocord_fixes.rs`: one test per fix to a Yapfast bug (above).
- Core `halocord::tests`: points 1 to 5 through a test plugin (each test thread
  has its own plugged list, so they don't meet).
- Core `halocord::staff::tests`: ranks, tags and options. `halocord::tests` also
  covers the fake voice state.
- Window `tests/settings.rs`: the plugins page, fake switches, the user area's dock.
- Window `tests/halocord_buttons.rs`: the button plugins (restart, read all, close all
  DMs, the counter, the member count).
- Window `halocord::tests`: point 5's rows, point 6's kinds, the stereo loader's
  saved values, and `every_hook_is_in_the_docs`, which fails when a Yapfast file
  calls into HaloCord or MochaCord and this page doesn't name it.
- Window `tests/halocord_menu_pictures.rs`: ignored tests that draw each right-click
  plugin's row into `audit/plugins/menus-<plugin>.png`.

## Candidates to offer upstream

- The microphone bit rate setting and stereo `Microphone` in `yapfast-discord`.
- The const-generic `Gate`.
- Push to talk's say for a block (copied into `flat.rs` from `Processing::pair`).
- The plug-in points, as a plugin layer for Yapfast.
