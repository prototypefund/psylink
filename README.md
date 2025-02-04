# PsyLink

[![License: GPL 3.0](https://img.shields.io/badge/license-GPL_v3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0.html)
[![Matrix Chatroom](https://img.shields.io/badge/chat-join%20us-green.svg)](https://matrix.to/#/#psylink:matrix.org)
[![Mastodon](https://img.shields.io/mastodon/follow/106535899613227044.svg?domain=https%3A%2F%2Ffosstodon.org&style=social)](https://fosstodon.org/@psylink)

Open source [EMG](https://en.wikipedia.org/wiki/Electromyography) neural interface.

[Website](https://psylink.me) ::
[Documentation](https://codeberg.org/psylink/psylink/wiki) ::
[Matrix Chatroom](https://matrix.to/#/#psylink:matrix.org) ::
[FAQ](https://psylink.me/faq)

![Photograph of PsyLink Prototype 10](https://psylink.me/img/prototypes/p10.jpg)

# Introduction

**Psylink** is intended as a human input device (like a keyboard) that predicts your
intention from muscle signals.  Using a neural network, trained through a
process where you press keys at the exact same time that you make movements
with the limb wearing the device, it learns correlations between the muscle
signals and intended keys, and can press them for you.

- [Video demos @ PeerTube](https://peertube.linuxrocks.online/video-channels/psylink/videos)
- [Development Log](https://psylink.me/blog)
- [Whats Next?](https://codeberg.org/psylink/psylink/wiki/Next-Steps)

# Software used to create/edit/run these files

- Schematics, PCB Layouts: KiCad 6.0 (some archived ones used 5.1.5)
- Arduino IDE 1.8.19
    - ArduinoBLE 1.2.1
    - Arduino\_LSM9DS1 1.1.0
- Python 3.8
    - For library versions, see the respective requirements.txt file
- GnuRadio 3.8.1.0

# Datasheet

This describes [prototype 4](https://psylink.me/p4).

![photo of the device](https://psylink.me/img/prototypes/p4.jpg)

- Features:
    - Battery-powered
    - Wireless, using Bluetooth Low Energy (BLE)
    - Supports 17 electrodes (8 pairs + 1 ground)
    - Transmits 8 signals at 8-bit resolution, 500Hz sampling rate
    - Linux graphical user interface for:
        - Mapping keyboard key presses to muscle signals
        - Training a neural network to predict key presses from signals
        - Simulating key presses based on neural network predictions
    - GNURadio integration for plotting the raw signals + FFT
- Hardware overview:
    - A fabric glove/sleeve with a central component (arduino + power supply)
      and 8 distributed signal clean-up units close to individual electrodes.
    - [KiCad Schematic](schematics/myocular0.5.1.sch)
    - [KiCad PCB of one distributed signal clean-up unit](schematics/myocular0.5.1d.kicad_pcb)
- Software dependencies:
    - Python3 (see [requirements.txt](python/requirements.txt))
        - [BLE-GATT](https://pypi.org/project/BLE-GATT/)
        - [matplotlib](https://matplotlib.org/)
        - [pynput](https://pypi.org/project/pynput/)
        - [tensorflow](https://www.tensorflow.org/)
        - [Pillow](https://python-pillow.org)
        - possibly the `python3-pil.imagetk` package
    - ArduinoIDE
        - [ArduinoBLE](https://www.arduino.cc/en/Reference/ArduinoBLE)
        - [Arduino LSM9DS1](https://www.arduino.cc/en/Reference/ArduinoLSM9DS1) (for accelerometer/gyroscope)
- Components:
    - 1x [Arduino Nano 33 BLE Sense](https://store.arduino.cc/arduino-nano-33-ble-sense)
    - 1x [LM324 Operational Amplifier](https://www.ti.com/product/LM324)
    - 8x [INA128 Instrumentational Amplifier](https://www.ti.com/product/INA128)
    - Any portable 6V power supply (like 2x CR2032)
    - 17x Electrodes, e.g. [Prym 390120](https://www.prym.com/en/non-sew-refill-for-390120-smooth-cap-10mm-silver-coloured-390104) (don't laugh, they work)
    - A forearm sleeve from non-conductive fabric
    - Additional standard components (see schematic)
- Power ratings
    - Supply Voltage: 4.5-6V
    - Power dissipation:
        - Idle: ~86.9mW (16.9mA x 5.14V)
        - Transmitting at 6-7kB/s: 92.5mW (18.0mA x 5.14V)
- Weight: ~85g (at 9.81m/s² gravitational acceleration)

# Acknowledgements

Very special thanks to every contributor. You all shaped the direction of this
project in your unique way, and we would not be at this point without you!

- [Roman Zimbelmann](https://rzim.dev): Founder and maintainer of PsyLink
- [Alex Lao](http://www.voltagedivide.com/): Consulting
- [AntonX](https://www.linkedin.com/in/anton-x/): Consulting, hardware design
- [Cade Brown](https://cade.site/): Testing, financial support
- [Fabien Devaux](http://github.com/fdev31/): Consulting
- [Farzam Khodajoo](https://www.linkedin.com/in/farzam-khodajoo): Consulting, hardware design
- [Hackaday.com](https://hackaday.com/2022/01/07/psylink-an-open-source-neural-interface-for-non-invasive-emg/): Spreading the word, networking
- [John Shahbazian](https://github.com/jshahbazi): Consulting, testing, software, financial support
- keks: Consulting, testing
- knoc: Consulting, testing
- [ktrask](https://twitter.com/ktrask23) ([mastodon](https://chaos.social/@ktrask)): Consulting, testing
- [PCBWay](https://pcbway.com): Sponsored production services
- [PerlinWarp](https://twitter.com/perlinwarp): Consulting, testing, documentation, networking, financial support
- [PrototypeFund](https://prototypefund.de): Consulting, financial support
- Ricard Collins: Consulting, financial support, testing
- [Wojciech Siewierski](https://einval.eu): Consulting
- Thanks to all the patrons on [Liberapay](https://liberapay.com/psylink/) for financial support!

# License

Copyright (C) 2024  Roman Zimbelmann

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License version 3 as
published by the Free Software Foundation.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
