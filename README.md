# Constellation

This simple Rust crate allows building a manipulable representation of a satellite constellation from a [constellation code](https://datatracker.ietf.org/doc/html/draft-piraux-space-constellation-code).

The main goal of this crate is computing every connectivity event possible between satellites and ground stations of a given constellation, for a specified duration and time step.

If a dump path is specified, the crate dumps the connectivity events seen by each ground station in a `json` file.
