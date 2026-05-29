use std::{collections::HashMap, f64};

pub mod constellation;
pub mod gateway;
pub mod shell;
pub mod walker;

pub const SPEED_OF_LIGHT: f64 = 299_792_458f64;

pub type Timestamp = f64;

/// Coordinates of a satellite within a shell (plane, rank).
pub type ShellCoordinates = (usize, usize);

pub type CrossPlaneDistances = HashMap<(ShellCoordinates, ShellCoordinates), f64>;
pub type CrossPlaneLatencies = HashMap<(ShellCoordinates, ShellCoordinates), f64>;
