# Yet-Another-BG3-Native-Mod-Loader
[![Build](https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader/actions/workflows/build.yml/badge.svg?event=push)](https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader/actions/workflows/build.yml)

This is a dll plugin mod loader for Baldur's Gate 3

It comes in 3 types:
- A background process which watches for bg3 to start, then transparently injects the plugins into it.
- A one time use separate injector tool which simply injects plugins into an already running game instance. (For those who do not want an always running app)
- A version which behaves exactly like Native Mod Loader. It automatically runs and patches bg3 when the game is started normally, then shuts down when the game shuts down. However, it requires (some fairly minimal) registry edits in order to accomplish this.

The main features of this mod loader are:
- No manual installation necessary\*!
- It does not modify any game files or touch the installation directory
  - Because it does not modify/replace/add files in the game directory, there is 0 maintenance required
  - 0 maintenance means, when you update the game, there is nothing to fix, the mod loader always works
  - You can keep all your game files pristine and untouched
- It is completely compatible with any NativeModLoader﻿ plugins
  - This means you can develop your plugin using my [Rust BG3 Plugin Template](https://github.com/MolotovCherry/Native-Plugin-Template-Rust) or [NativeModLoader](https://www.nexusmods.com/baldursgate3/mods/944)'s [BG3 Plugin Template](https://github.com/gottyduke/PluginTemplate)
- Compatible with steam and GoG
- Stores your plugins in the larian local data folder alongside the mod folder
- Does not install anything. Deleting the tool is the same as "uninstalling"\*

\* The injector and watcher do not require (un)installation. However, the autostart version requires (un)installation in the registry.

# Usage
For list of instructions, FAQ, and other info, please see the main [nexus mods page](https://www.nexusmods.com/baldursgate3/mods/3052)
