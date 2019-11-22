# SPREE prototype

This repository was created for initial experimentation around the shape and form of a SPREE
modules.

For an example of a SPREE module I decided to write a module that implements a trusted lamport clock.
A [lamport clock](https://en.wikipedia.org/wiki/Lamport_timestamps) is a simple algorithm that
allows distributed processes to build a partial ordering of a set of events.

Depending on whether we introduce some data from relay chain this might be or not be useful - relay
chain blocks should provide a global ordering for all events, IIUC.

For now, it is pretty simple and doesn't even feature a concept of ADC. The reason for that is that
this is the first iteration of this prototype and it is not clear what the constraints are.

