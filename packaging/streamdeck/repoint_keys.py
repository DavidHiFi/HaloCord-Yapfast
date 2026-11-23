"""Points the Stream Deck keys that used Elgato's Discord plugin and the old
screen share and fake voice plugins at the HaloCord for Yapfast plugin.
Stream Deck must be closed (it rewrites its profiles when it quits).

  python repoint_keys.py check     list what would change
  python repoint_keys.py apply     back up each page, then change it
"""
import datetime
import json
import os
import shutil
import sys

ROOT = os.path.join(os.environ["APPDATA"], "Elgato", "StreamDeck", "ProfilesV3")
PLUGIN = {"Name": "HaloCord for Yapfast", "UUID": "com.davidhifi.yapfast", "Version": "1.0.0.0"}
NAMES = {
    "mute": "Mute", "deafen": "Deafen", "camera": "Camera", "share": "Share Screen",
    "voicemode": "Voice Mode", "talk": "Push to Talk", "mutehold": "Push to Mute",
    "leave": "Leave Call", "screen1": "Screen 1 (Left)", "screen2": "Screen 2 (Main)",
    "screen3": "Screen 3 (Right)", "screen4": "Screen 4 (Top)", "fakemute": "Fake Mute",
    "fakedeafen": "Fake Deafen",
}
OLD = {
    "com.elgato.discord.mute": "mute",
    "com.elgato.discord.deafen": "deafen",
    "com.elgato.discord.videotoggle": "camera",
    "com.elgato.discord.streamtoggle": "share",
    "com.elgato.discord.pushtotalktoggle": "voicemode",
    "com.elgato.discord.pushto.talk": "talk",
    "com.elgato.discord.pushto.mute": "mutehold",
    "com.davidhifi.screenshare.display1": "screen1",
    "com.davidhifi.screenshare.display2": "screen2",
    "com.davidhifi.screenshare.display3": "screen3",
    "com.davidhifi.screenshare.display4": "screen4",
    "com.davidhifi.fakevoice.fakemute": "fakemute",
    "com.davidhifi.fakevoice.fakedeafen": "fakedeafen",
}


def new_id(action):
    uuid = action.get("UUID", "")
    if uuid == "com.elgato.discord.channel.voice":
        return "leave" if (action.get("Settings") or {}).get("server") == "[leave]" else None
    return OLD.get(uuid)


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "check"
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M")
    changed = 0
    for dirpath, _, files in os.walk(ROOT):
        if "manifest.json" not in files:
            continue
        path = os.path.join(dirpath, "manifest.json")
        try:
            with open(path, encoding="utf-8-sig") as f:
                data = json.load(f)
        except (OSError, ValueError):
            continue
        moves = []
        for controller in data.get("Controllers") or []:
            for spot, action in (controller.get("Actions") or {}).items():
                target = new_id(action)
                if target:
                    moves.append((controller["Actions"], spot, action, target))
        if not moves:
            continue
        page = os.path.relpath(dirpath, ROOT)
        for actions, spot, action, target in moves:
            print(f"{page} {spot}: {action['UUID']} -> com.davidhifi.yapfast.{target}")
            actions[spot] = {
                "ActionID": action.get("ActionID"),
                "LinkedTitle": True,
                "Name": NAMES[target],
                "Plugin": PLUGIN,
                "Resources": None,
                "Settings": {},
                "State": 0,
                "States": [{}, {}],
                "UUID": f"com.davidhifi.yapfast.{target}",
            }
            changed += 1
        if mode == "apply":
            shutil.copy2(path, f"{path}.bak-{stamp}")
            with open(path, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
    print(f"{changed} keys {'changed' if mode == 'apply' else 'to change'}")


if __name__ == "__main__":
    main()
