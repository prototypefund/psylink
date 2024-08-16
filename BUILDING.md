# Getting started

To build this software, [first install Rust and Cargo on your system.](https://doc.rust-lang.org/stable/cargo/getting-started/installation.html)

Then [download the source code](https://codeberg.org/psylink/psylink/archive/master.zip) and unpack it. All the following commands should be run in the root directory of the source code.

# Linux

Run this command:

    cargo run --release

# For Windows, cross-compiled on Linux

    cargo build --release --target x86_64-pc-windows-gnu

The executable file will be placed in the "target" subdirectory.

# For Android, cross-compiled on Linux

⚠️ PsyLink will start on Android but will NOT establish any bluetooth connections, so it's effectively useless ⚠️

First, set up the android SDK/NDK as described in the [Slint Android guide](https://snapshots.slint.dev/master/docs/rust/slint/android/)

These commands describe the instructions in arch linux, assuming that you have installed the package manager "yay" for Arch User Repositories (AUR) packages.

    pacman -S rustup jdk17-openjdk
    rustup target add aarch64-linux-android
    yay -S android-tools
    yay -S android-sdk android-sdk-build-tools android-sdk-platform-tools android-sdk-cmake android-sdk-cmdline-tools-latest
    yay -S android-platform-32
    yay -S android-ndk
    cargo install xbuild

Set the following environment variables:

1. `ANDROID_HOME` to the location of the android SDK. On ArchLinux, if installed via the `android-sdk` AUR package, this would be `/opt/android-sdk`.
2. `ANDROID_NDK_ROOT` to the location of the android NDK. On ArchLinux, if installed via the `android-ndk` AUR package, this would be `/opt/android-ndk`.
3. `JAVA_HOME` to the location of the Java compiler (`javac`) executable. On ArchLinux, if you have installed openjdk 17, this would be `/usr/lib/jvm/java-17-openjdk`. This is optional, if `javac` is in your `$PATH`.

Finally, run:

```
x build --platform android --arch arm64
```

You can run it on your Android device directly by

1. enabling developer mode on your Android device
2. allowing USB debugging in the Android developer settings
3. connecting the device with a cable to your computer
4. tap "accept" on the pop-up asking for whether to allow USB debugging for the connected computer
5. finding your android device ID with `x devices`
6. running this command, replacing `adb:ABCDEFGHI` with your device ID:

```
x run --device adb:ABCDEFGHI
```
