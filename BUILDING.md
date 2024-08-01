# Linux

    cargo build

# For Windows, cross-compiled on Linux

    cargo build --release --target x86_64-pc-windows-gnu

# For Android, cross-compiled on Linux

⚠️ WORK IN PROGRESS ⚠️

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

Finally, run: [TODO]

```
x build --platform android --arch arm64
```
