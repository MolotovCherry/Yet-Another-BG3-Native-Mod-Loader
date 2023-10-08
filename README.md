# Yet-Another-BG3-Mod-Loader
[![Build](https://github.com/MolotovCherry/Yet-Another-BG3-Mod-Loader/actions/workflows/build.yml/badge.svg?event=push)](https://github.com/MolotovCherry/Yet-Another-BG3-Mod-Loader/actions/workflows/build.yml)

This is a mod loader for Baldur's Gate 3

The main features of this mod loader are:
- It does not modify any original game files or touch the installation directory
- It does not need to be updated with any new BG3 releases (it stays working perpetually)
- It is a launcher (there are 2 executables for Vulkan or Dx11 launching, just like in BG3)
  - Which also means you can launch the game normally instead, and no dll plugins will load
- It is completely compatible with any [NativeModLoader](https://www.nexusmods.com/baldursgate3/mods/944) plugins
  - This means you can develop your plugin using the NativeModLoader framework, or any other (for example, [libmem](https://github.com/rdbo/libmem) for Rust)
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

# FAQ
### Virus warning!!! What! Virus?!
No, this is not a virus. The source code is right there for you to see and read. You can compile it yourself too if you want, it'll work the exact same way. This mod loader uses dll injection, and a lot of virus scanners just don't like that. This is the very thing that allows us to avoid modifying core game files!
### Halp!! All my saves/data are now missing and got deleted!
This mod loader _does not_ under any circumstances touch any of your data, saves, or profile(s). They _are not_ missing. Go to `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3\PlayerProfiles` and delete any `Debug` profiles you see. The profile you are most likely using is the Public one. For some reason, there's a bug with the game and it sometimes creates/launches into the debug profile, which makes it look like all your settings, saves, everything was suddenly deleted. They _are not_ gone! It will be visible just fine once you game properly loads the public profile again. If you are using steam, try to manually load steam/larian launcher first after deleting the Debug profile. It might work properly after that. I am hoping to get this reported soon so Larian will fix it asap.

### Where do I place my plugins?
Place your dll plugin files inside `C:\Users\<user>\AppData\Local\Larian Studios\Baldur's Gate 3\PlayerProfiles`

### Is this compatible with NativeModLoader plugins
100% compatible!

### Halp!! I used plugin `<insert name here>` and it did something bad/doesn't work!
This is not the mod loaders fault. The plugin itself needs to be fixed. You can report your bug/problem to the respective plugin authors page.

### Can you support loading plugins from `<gamefolder>/bin/NativeMods`?
I'm sorry, but I will not support that no. One of the core goals of this mod loader is to never ever, under any circumstances, touch or mess with the game's core installation. Supporting and allowing that would go against this central goal. You can move your plugins over to the plugin folder.
