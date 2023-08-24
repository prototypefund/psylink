# PsyLink Electrode Module 3.3

This board measures voltages on 8 electrodes, does basic analog processing, and outputs 4 cleaned and amplified signals destined for the analog pins of the power module.

The processing of the signal is the following, implemented 4 times on the circuit to produce 4 output signals:

    Electrode 1 -> Passive Highpass -.
                                      }==> Difference -> Amplifier -> Output Signal
    Electrode 2 -> Passive Highpass -'                   (gain: 251x)
                   (cut-off freq: 1.6Hz)

# Overview

- Part ID: ["b3.2"](https://psylink.me/b3.2/)
- Circuit ID: ["10.4"](https://psylink.me/c10.4/)
- Dimensions: 70 x 25mm
- Finalized on: 2023-06-16
- Tested: Yes
- Bill of materials: [LibreOffice .ods file](https://psylink.me/tables/bom_p10.ods)
- Components of interest:
    - [INA128](https://www.ti.com/product/INA128) Instrumentation Amplifier [(datasheet)](https://www.ti.com/lit/ds/symlink/ina128.pdf)
    - [M3 dome nuts, A2 stainless steel](https://www.schraubenking.at/M3-Hutmutter-DIN1587-Edelstahl-A2-P002263), as electrodes
- Known bugs:
    - Excessive interference on the trace between "U2" and "J3" ("out2"), resulting in a disturbed signal 2.  This mostly disappears when the module is pressed firmly onto the skin.

# Main Features

- 8 Input Signals from screw-mounted electrodes on EL1-EL8 (e.g. [M3 dome nuts of A2 stainless steel](https://www.schraubenking.at/M3-Hutmutter-DIN1587-Edelstahl-A2-P002263)
- 4 Output Signals on J3, cleaned and amplified, between 0V and V+ (usually 5V)
- Differential amplification of the signals with a gain of 251x using [INA128](https://www.ti.com/product/INA128) Instrumentation Amplifiers
- High-pass filtering using a passive highpass filter with a cut-off frequency of below 1.6Hz
- Requires a power supply on either J1 (or J2).
    - V+: 5V
    - Vref: 2.5V (must be connected to the ground electrode touching the skin)
    - GND: 0V
- Supports daisy-chaining the power supply to another electrode module via J2 (or J1 if you used J2 for the power input already)
- Supports plugging in external electrodes via dupont connectors on pins EX1-EX8, though it is advisable to keep the cable to the electrodes as short as possible (<10cm)
- Provides mounting points for 8 dome nuts (e.g. [M3 dome nuts of A2 stainless steel](https://www.schraubenking.at/M3-Hutmutter-DIN1587-Edelstahl-A2-P002263)) that act as ground electrode and spacers between the board and the skin
- Provides solder jumpers JP1-JP8 (closed by default) to disable the built-in screw-mounted electrodes EL1-EL8 in favor of external electrodes on EX1-EX8, so that you can wear the board on the skin without disturbing the external electrode signals with the built-in electrodes.

# IMPORTANT ASSEMBLY INSTRUCTIONS

- Before soldering on any through-hole components, their pins must be shortened with e.g. wire cutters so that they do not extend out of the board on the bottom side, to avoid scratching the skin
- The "Vref" pin must be connected to a ground electrode on the skin.  The standard method is:
    1. connect the Vref pin of the electrode module with the Vref pin of the power module
    2. wear the power module on the skin so that the screws are firmly touching the skin
    3. ensure that one of the screws is actually enabled as ground electrode by reading the assembly instructions of the power module

# PCB Images

![Front Side](https://psylink.me/img/boards/b3.2.png)

![Back Side](https://psylink.me/img/boards/b3.2_back.png)

# Circuit Image

![Circuit](https://psylink.me/img/circuits/c10.4.png)

# Changelog
## Circuit

Electrode Module 3.3:

- Fixed aggressive filtering of signals by changing passive highpass filter capacitors C1-C8 from 100pF to 100nF.

Electrode Module 3.2:

- Gain resistors R9, R11, R12 have been reduced from 1000 Ohms to 100 Ohms, raising the gain tenfold, from 51x to 501x.
- Gain resistor R10 was only reduced to 200 Ohms, raising the gain to only 251x. This was done to offset an interference bug along the out2 trace.  Once this is fixed, the resistance should be set to equal that of R9/R11/R12 once again.
- JP1-JP8 are now closed by default, to save time during assembly

## PCB

Electrode Module 3.3:

- No changes.

Electrode Module 3.2:

- Moved EX1 closer to EX5 and EX4 closer to EX8, so that two 2-pin-headers can be used instead of four 1-pin headers
- Longer solder pads on U1-U4 for easier hand-soldering
- Improved labels, added PsyLink logo
