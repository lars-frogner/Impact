# Tracy setup

## Build

### Install dependencies:

#### Ubuntu with X11

```bash
sudo apt install cmake clang git libcapstone-dev xorg-dev dbus libgtk-3-dev
```

#### Ubuntu with Wayland

```bash
sudo apt install libglfw-dev libgtk-3-dev libfreetype6-dev libtbb-dev debuginfod libwayland-dev dbus libxkbcommon-dev libglvnd-dev meson cmake git wayland-protocols
```

#### Arch

```bash
pacman -Syu --noconfirm && pacman -S --noconfirm --needed freetype2 tbb debuginfod wayland dbus libxkbcommon libglvnd meson cmake git wayland-protocols
```

### Clone repo

```bash
git clone https://github.com/wolfpld/tracy.git
cd tracy
git checkout v0.13.0 # Compatible with current `tracy-client` version (0.18.3)
```

### Compile

#### X11

```bash
cmake -S profiler -B profiler/build -DCMAKE_BUILD_TYPE=Release -DLEGACY=ON
cmake --build profiler/build -j"$(nproc)"
```

#### Wayland

```bash
cmake -S profiler -B profiler/build -DCMAKE_BUILD_TYPE=Release
cmake --build profiler/build -j"$(nproc)"
```

The compiled binary is `profiler/build/tracy-profiler`.
