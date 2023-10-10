# Yet-Another-BG3-Mod-Loader
[![Build](https://github.com/MolotovCherry/Yet-Another-BG3-Mod-Loader/actions/workflows/build.yml/badge.svg?event=push)](https://github.com/MolotovCherry/Yet-Another-BG3-Mod-Loader/actions/workflows/build.yml)

This is a native mod loader for Baldur's Gate 3

The main features of this mod loader are:
- It does not modify any original game files or touch the installation directory
- It does not need to be updated with any new BG3 releases (it stays working perpetually)
- It is a launcher (there are 2 executables for Vulkan or Dx11 launching, just like in BG3)
  - Which also means you can launch the game normally instead, and no dll plugins will load
- It is completely compatible with any [NativeModLoader](https://www.nexusmods.com/baldursgate3/mods/944) plugins
  - This means you can develop your plugin using my [Rust BG3 Plugin Template](https://github.com/MolotovCherry/BG3-Plugin-Template-Rust) or [NativeModLoader](https://www.nexusmods.com/baldursgate3/mods/944)﻿﻿﻿'s [BG3 Plugin Template](https://github.com/gottyduke/BG3_PluginTemplate)﻿﻿
- Has the ability to disable/enable any plugins in the config (useful for any mod managers)
- Stores your plugins in the larian local data folder alongside the mod folder*

\* `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3\Plugins`

# How to use
1. Download the latest release from [releases](https://github.com/MolotovCherry/Yet-Another-BG3-Mod-Loader/releases)
2. Place the `bg3.exe` and `bg3_dx11.exe` files wherever you want, maybe create shortcuts to them even
3. Run one of the launchers. You'll get a finish setup message the first time, read it and follow the instructions.
4. **Read the FAQ. It's important!**

# Config options
| Option | Description |
|-------------|------------|
| `install_root` | The game's root installation directory, e.g. `C:\Program Files (x86)\Steam\steamapps\common\Baldurs Gate 3` |
| `flags` | Extra command line flags to pass to the game upon startup |
| `steam` | Use steam to launch the game, recommended leaving this enabled. If disabled, will directly launch the game exe, may launch the game twice |
| `disabled` | An array of plugins to disable. Each entry is the plugins filename without extension, e.g. `FooBar.dll` should have an entry named `FooBar` |

# Building
- [Install Rust](https://rustup.rs/)
- Install [Visual Studio](https://visualstudio.microsoft.com/downloads/), build tools, and Desktop Development with C++
- Run `cargo build` or `cargo build --release`

# Making plugins
You can use my [Rust BG3 Plugin Template](https://github.com/MolotovCherry/BG3-Plugin-Template-Rust) or [NativeModLoader](https://www.nexusmods.com/baldursgate3/mods/944)'s [BG3 Plugin Template](https://github.com/gottyduke/PluginTemplate). What you use doesn't really matter much, just as long as it's a dll with a `DllMain` that does its hooking at runtime.

# FAQ
### Virus warning!!! Why?!
No, this is not a virus. This mod loader uses dll injection, and virus scanners or browser download might not like that. This is the feature that allows us to avoid modifying core game files! The source code is freely available to all, and you may also compile it yourself using the build instructions.

### I got a smartscreen warning, how do I remove it?
If the files are not widely distributed yet, you may also receive a smartscreen popup on windows when you run it. You can remove it by following [these instructions﻿](https://www.windowscentral.com/how-disable-smartscreen-trusted-app-windows-10).

### Halp!! All my saves/data are now missing and got deleted!
This mod loader _DOES NOT_ under ANY circumstances touch your game files, game data, or game saves/profile(s)*. They _are not_ missing. For some reason, there's a bug with the game and it sometimes creates/launches into the debug profile, which makes it look like all your settings, saves, everything was suddenly deleted. They _are not_ gone! Go to `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3\PlayerProfiles` and delete any `Debug` profiles you see. The profile you are most likely using is the Public one. It will be visible just fine once your game properly loads the public profile again. If you are using steam, try to manually load steam/larian launcher first after deleting the Debug profile. It might work properly after that. I hope Larian fixes this bug soon

\* `C:\Users\<user>\AppData\Local\Larian Studios\Launcher\Settings\preferences.json` is the only file touched. This is required because this file must be set to load in vulkan or dx11 mode, and the game won't start in the right mode otherwise. It's worth noting that larian's own launcher does the exact same thing.

### Where do I place my plugins?
Place your dll plugin files inside `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3\PlayerProfiles`

### How do I uninstall?
This program does not install itself. To "uninstall" it, simply delete the launchers you downloaded.

### Is this compatible with NativeModLoader plugins
100% compatible! Just as the mod didn't rely on any NativeModLoader specific behavior (if so, file an issue with the plugin author!)

### Halp!! I used plugin `<insert name here>` and it did something bad/doesn't work!
This is not the mod loaders fault. The plugin itself needs to be fixed. You can report your bug/problem to the respective plugin authors page.

### Can you support loading plugins from `<gamefolder>/bin/NativeMods`?
I'm sorry, but I will not support that no. One of the core goals of this mod loader is to never ever, under any circumstances, touch or mess with the game's core installation. Supporting and allowing that would go against this central goal. You can move your plugins over to the plugin folder.
