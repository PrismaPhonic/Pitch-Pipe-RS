![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Released API docs](https://docs.rs/pitch-pipe/badge.svg)](https://docs.rs/pitch-pipe)

# ** DO NOT USE THIS CRATE RIGHT NOW**

After extensive testing, it seems that this does not meet the desired lag requirements set by the user AND over aggressively smooths data. I can only assume that either the 60 hz table from the parent repo is not appropriate for all 60 hz signals, or the study data was fabricated. Please don't waste as much time on this as I did. I will update this if I get it sorted. 

# Pitch Pipe

This is a rust port of Pitch Pipe. The original researchers who invented Pitch
Pipe put up a sample repo in javascript that this was ported from. 

[Original repo here](https://github.com/ISUE/PitchPipe)

From their README:

Pitch Pipe is a custom low-pass filter calibration technique that finds optimal
parameters based on context specific information. As such, Pitch Pipe requires
three inputs: a signal from which to derive relevant characteristics, a
low-pass filter to calibrate, and an application specific criteria to optimize
for, namely precision and lag. From the input device signal, Pitch Pipe
extracts noise and maximum user speed estimates, which we use to generate
synthetic noise and edge patterns. Thereafter, Pitch Pipe performs a grid
search over the filter’s parameter space, evaluating its performance on the
stated synthetic data. Pitch Pipe finally outputs the parameter set that best
matches the application specific criteria. As such, Pitch Pipe internally
comprises three steps that are to estimate noise, estimate maximum user speed,
and optimize the parameter set.

## Caveats

This was ported mostly 1:1 from the parent repo with some changes:
1. This is designed to take three axis instead of two, x, y and z. This was
   done because I personally use this to tune one euro filters that are used to
   smooth out accelerometer data long three axis.
2. The original implementation involved a large Calibration function/object
   with state changes to transition from one state to the next, with a lot of
   values initialized to null at the start. I initially ported it this way and
   realized that it made for fragile code, where if each stage wasn't
   instantiated correctly we could end up in very weird places. I instead
   refactored it using a builder pattern so each stage of calibration is clear,
   and to minimize optional values as much as possible.
3. You'll notice a hard coded table that is used to calculate precision. I
   asked about this in the original repo and was told, "That is a grid
   representing the parameter space for the filter for 60 hz data. It covers
   fixed ranges of values for jitter, cutoff, and beta. I believe this should
   enable appropriate search for parameters fitting any 60 hz signal." I'm
   currently in the process of trying to figure out how to generate a grid like
   this for other frequencies. For the time being, this means that this port,
   like the parent it's ported from, only supports 60 hz signals. Please open a
   PR if you know how to make this more general.

## Research Paper

[Pitch Pipe Paper](http://graphicsinterface.org/proceedings/gi2019/gi2019-27/)
