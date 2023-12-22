# `mate-rov-2024`

This project is a custom software stack to control ROVs based on a Raspberry Pi 4.
This project It is based on the bevy game engine and is intended to be used in the MATE ROV Competition.

## ROV Hardware Support

The software current only has support for the following hardware

- Raspberry Pi 4
- The Blue Robotics Navigator Flight Controller
  - ICM20602 (6-axis IMU, Gyro + Accelerometer)
  - MMC5983 (3-axis Magnetometer)
  - PCA9685 (16-channel PWM controller)
    - Controls PWM based ESCs to drive thrusters
- The Blue Robotics Bar30 and Bar02 Depth Sensors
  - MS5837 (Depth sensor)
- Neopixel Light Strips
  - The RGB kind
- Any H.264 webcam

Hardware support may be expanded in the future, but this is not currently a priority.

## Surface Hardware Support

This code has only been tested on a Framework 13in Laptop 12th gen running Gentoo and using the Wayland compositor Hyprland (must drive ROV with style).
Most other configurations should work provided the correct gstreamer plugins and system libraries are available.

See [Bevy Linux Deps](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)\
See [opencv-rust Deps](https://github.com/twistedfall/opencv-rust)\
OpenXR: media-libs/openxr-loader
TODO: Document gstreamer deps

## Motor configurations

Unlike most other projects, we support literally every motor configuration provided the following data is available.

- Thruster Performance curves
  - Needs a mapping between PWM, thrust, and amperage draw.
  - This is available for Blue Robotics T200 thrusters.
  - This requirement may be removed in the future
- Thruster Position Information
  - Orientation as a vector
  - Position relative to the robot's origin as a vector

Our motor code is dynamic, simple, correct, and fast (just one matrix multiplication).
Thruster data can be modified in real time if needed, and when a single solution is not possible, the best solution is used.
See the `motor_code` crate for more.

## Project Structure

This code base is broken up into the following crates

- `robot`
  - This is the binary running on the Raspberry Pi
  - Actually manages/controls the robot
  - It is written as a headless bevy app
- `surface`
  - This is the binary running on the laptop controling the ROV
  - Connects to the ROV, reads human input, displays cameras, runs computer vision
  - Written as a normal bevy app
- `common`
  - This library defines the communication between `robot` and `surface`
  - ECS sync, ECS bundles and components, most type definitions, networking protocol
- `motor_code`
  - This library implements the secret sauce motor_code
  - The not-so-heavy lifting behind how be map movement commands to thruster commands
- `networking`
  - This library implements a fast non-blocking TCP server and client
  - Handles low level protocol details
- `runner-rpi`
  - This binary automates uploading builds of `robot` to the Raspberry Pi over ssh
  - This should have been a bash script, but now it's blazingly fast

## System Ordering

- Startup: Setup what's needed
  - Add necessary data to ECS
- First: Prepare for tick
  - Currently unused
- PreUpdate: Read in new data
  - Read inbound network packets, sensors, user input
- Update: Process state and determine next state
  - Compute new movement, motor math, compute orientation
- PostUpdate Write out new state
  - Write outbound network packets, motor speeds, handle errors
  - Avoid mutating state
- Last: Any cleanup
  - Shutdown logic

## Sync Model

### Background

Fundamental premise: Bevy ECS is perfect

In our previous codebase (Eoghanmc22/mate-rov-2023) the surface and robot implementation were fundamentally different.

- Different core data structure (ECS vs Type erased hash map)
- Different programming paradigms (Data driven vs Event based message passing)
- Different concurrency models (Concurrent game loop vs Every subsystem gets its own thread)
- Probably other things

This worked but made communication and state harder to maintain and uglier.
It also lead to strange limitations such as the opencv thread not having access to the robot's state.
Also, we couldn't do goofy things like drive two ROVs at the same time because both ROVs would try to use the same keys in the "distributed" hash map.
Armed with the perfect excuse to rewrite everything, I settled on the idea of a distributed ECS.
This would allow communication between `surface` and `robot` to transparent as synchronization would simply be implemented upon the same infrastructure already used to store local state.
Furthermore, all the (somewhat ugly) sync logic is contained within a single module in `common` instead of being spread throughout the codebase at every state read/write.
This allows for a consistent code style between `robot` and `surface` and a general simplification of the codebase.

### Design

A list of component types implement serde's Serialize and Deserialize traits and entities with any of these components will be replicated on all peers
We take advantage of bevy's change detection system and send a packet to all peers when a component with a known type is mutated.
This will update the peer's replicated entity to match the updated value for the component.

TODO: Explain how it works after I rewrite it

## Thanks

Thanks to all the people who made the libraries and tools I used in ways they could have never imagined.

Made with :heart: in :crab: :rocket: 
