use std::marker::PhantomData;

use constellation_code::Code;
use satkit::{
    Duration, ITRFCoord, Instant, Vector3, consts::EARTH_RADIUS, kepler::Kepler,
    prelude::qgcrf2itrf,
};

use crate::{
    CrossPlaneDistances, CrossPlaneLatencies, SPEED_OF_LIGHT, ShellCoordinates, Timestamp,
};

fn altitude_to_semi_major_axis(a: f64) -> f64 {
    EARTH_RADIUS + a
}

#[derive(Clone, Debug)]
/// Structure representing a single satellite.
pub struct Satellite {
    plane: usize,
    rank: usize,
    // altitude: f64,
    // inclination: f64,
    // raan: f64,
    // true_anomaly: f64,
    kepler: Kepler,
}

impl Satellite {
    /// Create a new Satellite object.
    ///
    /// # Arguments
    ///
    /// - `rank` - The satellite rank within an orbital plane.
    /// - `plane` - The satellite orbital plane within a shell.
    /// - `altitude` - The satellite altitude w.r.t. the Earth's sea level (meters).
    /// - `inclination` - The satellite incliniation (degrees).
    /// - `raan` - Right Ascension of the Ascending Node (degrees).
    /// - `true_anomaly` - Anomaly type representing location of satellite along
    ///  the orbital plane (degrees).
    ///
    /// # Returns
    /// - `Satellite` - A new Satellite object.
    pub fn new(
        rank: usize,
        plane: usize,
        altitude: f64,
        inclination: f64,
        raan: f64,
        true_anomaly: f64,
    ) -> Self {
        let kepler = Kepler {
            a: altitude_to_semi_major_axis(altitude),
            eccen: 0f64,
            incl: inclination.to_radians(),
            raan: raan.to_radians(),
            w: 0f64,
            nu: true_anomaly.to_radians(),
        };
        Self {
            rank,
            plane,
            // altitude,
            // inclination,
            // raan,
            // true_anomaly,
            kepler,
        }
    }

    /// Determine the new position of the satellite after `ts` seconds since
    /// the last timestamp.
    ///
    /// # Arguments
    ///
    /// - `ts` - The timestamp at which the position is computed (seconds).
    pub fn simulate(&mut self, ts: Timestamp) {
        self.kepler = self.kepler.propagate(&Duration::from_seconds(ts))
    }

    /// Return the satellite position in cartesian coordinates (in meters).
    pub fn position(&self) -> Vector3 {
        self.kepler.to_pv().0
    }

    /// Return the distance between the current satellite and the `s` satellite.
    pub fn distance(&self, s: &Self) -> f64 {
        (self.position() - s.position()).norm()
    }

    /// Return the satellite position in shell coordinates.
    pub fn coord(&self) -> ShellCoordinates {
        (self.plane as usize, self.rank as usize)
    }

    /// Return the satellite position in ITRF coordinates.
    pub fn itrf_coord(&self, ts: Timestamp) -> Result<ITRFCoord, satkit::itrfcoord::Error> {
        let ts =
            Instant::from_datetime(2026, 1, 1, 11, 0, 0f64).unwrap() + Duration::from_seconds(ts);
        let trans = qgcrf2itrf(&ts);
        let raw_pos = trans * self.position();
        ITRFCoord::from_slice(raw_pos.as_slice())
    }

    /// Return the orbital period of the satellite (seconds).
    pub fn period(&self) -> f64 {
        self.kepler.period()
    }
}

#[derive(Debug, Clone)]
/// Structure representing an orbital plane.
pub struct Orbit {
    // idx: u64,
    // nsats: usize,
    // altitude: f64,
    // inclination: f64,
    // raan: f64,
    // phase: f64,
    /// Satellites contained in the orbital plane.
    pub satellites: Vec<Satellite>,
}

impl Orbit {
    pub fn new(
        idx: u64,
        nsats: usize,
        altitude: f64,
        inclination: f64,
        raan: f64,
        phase: f64,
    ) -> Self {
        let satellites = (0..nsats)
            .into_iter()
            .map(|sat_idx| {
                Satellite::new(
                    sat_idx,
                    idx as usize,
                    altitude,
                    inclination,
                    raan,
                    phase + (360f64 * sat_idx as f64 / nsats as f64),
                )
            })
            .collect();

        Self {
            // idx,
            // nsats,
            // altitude,
            // inclination,
            // raan,
            // phase,
            satellites,
        }
    }

    /// Determine the new position of the satellites belonging to the orbital
    /// plane after `ts` seconds since the last timestamp.
    ///
    /// # Arguments
    ///
    /// - `ts` - The timestamp at which the position is computed (seconds).
    pub fn simulate(&mut self, ts: Timestamp) {
        self.satellites.iter_mut().for_each(|s| s.simulate(ts));
    }

    /// Return the position of each satellite belonging to the orbital plane
    /// in cartesian coordinates (meters).
    /// The index in the vector corresponds to the satellite rank.
    pub fn position(&self) -> Vec<Vector3> {
        self.satellites.iter().map(|s| s.position()).collect()
    }
}

/// Meta-trait representing a shell pattern.
/// Actual patterns **must** implement this trait.
pub trait Pattern
where
    Self: 'static + std::fmt::Debug + Sized + Neighbors<Self>,
{
}

/// Simple trait specifying how a shell with a given pattern re
pub trait Neighbors<P>
where
    P: Pattern,
{
    /// Return the neighbors of the satellite with `coords` within `shell`.
    /// The list of neighbors depends on the shell pattern.
    fn neighbors(shell: &Shell<P>, coords: ShellCoordinates) -> Vec<ShellCoordinates>;
}

#[derive(Debug, Clone)]
/// Basic shell metrics.
pub struct ShellMetrics {
    /// Shell's orbital period.
    pub orbital_period: f64,
    /// Shell's in-plane distance.
    pub ip_distance: f64,
    /// Shell's in-plane latency.
    pub ip_latency: f64,
}

#[derive(Debug, Clone)]
/// Structure representing a constellation shell following `Pattern`.
pub struct Shell<Pattern> {
    /// The shell code.
    pub code: Code,
    /// The number of orbital planes within the shell.
    pub norbits: usize,
    /// Total number of satellite within the shell.
    pub nsats: usize,
    /// Number of satellite per orbital plane within the shell.
    pub nsats_per_orbit: usize,
    /// Shell altitude w.r.t. the Earth sea-level (meters).
    pub altitude: f64,
    pub inclination: f64,
    pub phasing: u64,
    pub raan: f64,
    /// FIXME: Not used for now.
    pub full_connect: bool,
    /// Basic shell metrics.
    pub metrics: ShellMetrics,
    /// List of orbital planes.
    pub orbits: Vec<Orbit>,
    _pattern: PhantomData<Pattern>,
}

impl<P> Shell<P>
where
    P: Pattern,
{
    pub fn new(
        code: Code,
        norbits: usize,
        nsats: usize,
        altitude: f64,
        inclination: f64,
        phasing: u64,
        raan: f64,
    ) -> Result<Self, ()> {
        // TODO: Sanity check for nsats_per_orbit
        let nsats_per_orbit = nsats / norbits;

        // Determine orbits.
        let mut orbits: Vec<Orbit> = (0..norbits)
            .map(|idx| {
                Orbit::new(
                    idx as u64,
                    nsats_per_orbit,
                    altitude,
                    inclination,
                    raan * idx as f64 / norbits as f64,
                    idx as f64 * phasing as f64 * 360f64 / nsats as f64,
                )
            })
            .collect();

        // Compute basic metrics.
        let (sat0, sats) = orbits[0].satellites.split_at_mut(1);
        let sat0 = &mut sat0[0];
        let sat1 = &mut sats[0];
        let orbital_period = sat0.period();
        let ip_distance = sat0.distance(sat1);
        let ip_latency = ip_distance / SPEED_OF_LIGHT * 1000f64;
        let metrics = ShellMetrics {
            orbital_period,
            ip_distance,
            ip_latency,
        };

        let shell = Self {
            code,
            norbits,
            nsats,
            nsats_per_orbit,
            altitude,
            inclination,
            phasing,
            raan,
            // FIXME
            full_connect: true,
            metrics,
            orbits,
            _pattern: Default::default(),
        };
        Ok(shell)
    }

    /// Determine the new position of the satellites belonging to the shell
    /// after `ts` seconds since the last timestamp.
    ///
    /// # Arguments
    ///
    /// - `ts` - The timestamp at which the position is computed (seconds).
    pub fn simulate(&mut self, ts: Timestamp) {
        self.orbits.iter_mut().for_each(|o| o.simulate(ts));
    }

    /// Return a reference to the `Satellite` identified by the coordinates
    /// `(plane, rank)`, else `None` if the coordinates do not belong to the shell.
    pub fn get_sat(&self, (plane, rank): ShellCoordinates) -> Option<&Satellite> {
        self.orbits.get(plane).and_then(|o| o.satellites.get(rank))
    }

    /// Return a mutable reference to the `Satellite` identified by the coordinates
    /// `(plane, rank)`, else `None` if the coordinates do not belong to the shell.
    pub fn get_sat_mut(&mut self, (plane, rank): ShellCoordinates) -> Option<&mut Satellite> {
        self.orbits
            .get_mut(plane)
            .and_then(|o| o.satellites.get_mut(rank))
    }

    /// Compute distances between cross-plane neighboring satellites within the
    /// shell.
    pub fn get_cross_plane_distances(&self) -> CrossPlaneDistances {
        let mut ret = CrossPlaneDistances::default();
        self.orbits.iter().enumerate().for_each(|(oidx, o)| {
            o.satellites.iter().enumerate().for_each(|(sidx, s)| {
                let coords = (oidx, sidx);
                // FIXME if star
                let distances: CrossPlaneDistances = P::neighbors(self, coords)
                    .iter()
                    .filter_map(|&(oidx2, sidx2)| {
                        if oidx != oidx2
                            && let Some(s2) = self.get_sat((oidx2, sidx2))
                        {
                            Some(((coords, (oidx2, sidx2)), s.distance(s2)))
                        } else {
                            None
                        }
                    })
                    .collect();
                ret.extend(distances);
            });
        });
        ret
    }

    /// Compute latencies between cross-plane neighboring satellites within the
    /// shell.
    pub fn get_cross_plane_latencies(&self) -> CrossPlaneLatencies {
        self.get_cross_plane_distances()
            .iter()
            .map(|(&(s1, s2), dist)| ((s1, s2), dist / SPEED_OF_LIGHT * 1000f64))
            .collect()
    }
}
