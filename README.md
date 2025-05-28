# Impact

This is a highly experimental and work-in-progress game engine written in Rust. It is by no means featured enough for building games yet (except for very boring ones).

## Features

### Graphics

The engine does physically based 3D rendering. The graphics system is built on [`wgpu`](https://github.com/gfx-rs/wgpu). Notable graphics capabilities include:

- A general microfacet material model.
- Area lights with soft shadows.
- Normal mapping and parallax mapping.
- Ambient lighting and ambient occlusion.
- Fully HDR rendering with bloom, automatic exposure and tone mapping.
- Temporal anti-aliasing.

### Physics

- Various integration schemes for motion (including 4'th order Runge-Kutta).
- Full rigid body dynamics for objects of any shape.
- Various force and torque generators.
- A drag model that computes both forces and torques for objects of any shape.
- Collision resolution using sequential impulses.

### Voxels

Voxel based objects have first-class support. The shape of these objects are defined by signed distance fields, which combined with Surface Nets-based meshing gives them a smooth appearance. An object can be comprised of voxels with many different materials, which are smoothly blended between during rendering. The objects can also be arbitrarily deformed or split up, which dynamically affects their physical behavior. See the video below for a demonstration.

### ECS

The engine uses its own Entity-Component-System (ECS) implementation, which also serves as the main public API.

### Scripting

There is currently limited scripting support using the [Roc](https://www.roc-lang.org/) language. Roc is a very young language, but it shows great promise both for standalone applications and, thanks to its [platform](https://www.roc-lang.org/platforms) concept, for being embedded into larger systems. Since it compiles to machine code, it has the potential to be a very performant scripting language. The [How applications work](#how-applications-work) section describes how Roc is embedded into the Impact engine. At the moment, Roc scripts are responsible for scene setup and input handling.

## Planned features

In rough order of priority (which may change):

- GUI.
- Collision detection for voxel objects.
- N-body gravity simulation.
- More options for procedural voxel generation.
- Expanded scripting capabilities.
- Audio.
- Let's see when we get here...

## Showcase

![voxel_asteroid](showcase/voxel_asteroid.gif "Voxel Asteroid")

![screenshot_1](showcase/screenshot_1.png "Screenshot 1")

![screenshot_2](showcase/screenshot_2.png "Screenshot 2")

## How applications work

An Impact application has three components: the Rust application crate, the Roc platform and the Roc script.

**The application crate**

This contains all application-specific functionality outside of scripting. It links statically with the engine crate and together they form a shared library. This shared library is dynamically loaded by the Roc platform at runtime.

The app crate exposes a scripting API via FFI – this provides operations like creating entities or executing engine commands. It also expects certain Roc callbacks (for tasks like scene setup and input handling) to be implemented by the script, which it calls via dynamically loaded FFI functions.

**The Roc platform**

The Roc platform consists of a small Rust crate and a Roc package.

- The **platform crate** is compiled as a static library and linked into the Roc script. It forwards API calls from the script to the application+engine shared library, which it loads at runtime using dynamic linking. It also provides the required low-level functions for Roc's runtime (e.g., memory allocation and I/O).

- The **platform Roc package** defines the Roc-side API for interacting with the engine. It provides ergonomic wrappers around raw FFI functions and declares the callback interface that the app crate expects Roc scripts to implement.

**The Roc script**

This is a standard Roc application that imports the platform package. But rather than having a `main` function, it implements the callback functions required by the platform, and during the execution of these functions it uses the scripting API to modify the application and engine state. When compiled, the script is statically linked with the platform crate, producing a shared library that the application crate can load and interact with.
