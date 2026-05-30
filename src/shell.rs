use std::marker::PhantomData;

use satkit::{
    Duration, ITRFCoord, Instant, Vector3, consts::EARTH_RADIUS, kepler::Kepler,
    prelude::qgcrf2itrf,
};

use tracing::info;

use crate::{
    CrossPlaneDistances, CrossPlaneLatencies, SPEED_OF_LIGHT, ShellCoordinates, Timestamp,
    code::Code,
};

#[derive(Clone, Debug)]
pub struct Satellite {
    orbit: u64,
    idx: u64,
    altitude: f64,
    inclination: f64,
    raan: f64,
    true_anomaly: f64,
    pub kepler: Kepler,
}

fn altitude_to_semi_major_axis(a: f64) -> f64 {
    EARTH_RADIUS + a
}

impl Satellite {
    pub fn new(
        idx: u64,
        orbit: u64,
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
            idx,
            orbit,
            altitude,
            inclination,
            raan,
            true_anomaly,
            kepler,
        }
    }

    pub fn simulate(&mut self, ts: Timestamp) {
        self.kepler = self.kepler.propagate(&Duration::from_seconds(ts))
    }

    pub fn position(&self) -> Vector3 {
        self.kepler.to_pv().0
    }

    pub fn distance(&self, s: &Self) -> f64 {
        (self.position() - s.position()).norm()
    }

    pub fn coord(&self) -> ShellCoordinates {
        (self.orbit as usize, self.idx as usize)
    }

    pub fn itrf_coord(&self, ts: Timestamp) -> Result<ITRFCoord, satkit::itrfcoord::Error> {
        let ts =
            Instant::from_datetime(2026, 1, 1, 11, 0, 0f64).unwrap() + Duration::from_seconds(ts);
        let trans = qgcrf2itrf(&ts);
        // info!(trans=?trans.to_axis_angle(), kep=?self.position());
        let raw_pos = trans * self.position();
        // info!(?raw_pos);
        ITRFCoord::from_slice(raw_pos.as_slice())
    }
}

#[derive(Debug, Clone)]
pub struct Orbit {
    idx: u64,
    nsats: usize,
    altitude: f64,
    inclination: f64,
    raan: f64,
    phase: f64,
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
                    sat_idx as u64,
                    idx,
                    altitude,
                    inclination,
                    raan,
                    phase + (360f64 * sat_idx as f64 / nsats as f64),
                )
            })
            .collect();

        Self {
            idx,
            nsats,
            altitude,
            inclination,
            raan,
            phase,
            satellites,
        }
    }

    pub fn simulate(&mut self, ts: Timestamp) {
        self.satellites.iter_mut().for_each(|s| s.simulate(ts));
    }

    pub fn position(&self) -> Vec<Vector3> {
        self.satellites.iter().map(|s| s.position()).collect()
    }
}

pub trait Pattern
where
    Self: 'static + std::fmt::Debug + Sized + Neighbors<Self>,
{
}

pub trait Neighbors<P>
where
    P: Pattern,
{
    fn neighbors(shell: &Shell<P>, coords: ShellCoordinates) -> Vec<ShellCoordinates>;
}

#[derive(Debug, Clone)]
pub struct ShellMetrics {
    pub orbital_period: f64,
    pub ip_distance: f64,
    pub ip_latency: f64,
}

#[derive(Debug, Clone)]
pub struct Shell<Pattern> {
    pub norbits: usize,
    pub nsats: usize,
    pub nsats_per_orbit: usize,
    pub altitude: f64,
    pub inclination: f64,
    pub phasing: u64,
    pub raan: f64,
    pub full_connect: bool,
    pub metrics: ShellMetrics,
    pub orbits: Vec<Orbit>,
    _pattern: PhantomData<Pattern>,
}

impl<P> Shell<P>
where
    P: Pattern,
{
    pub fn new(
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
        let orbital_period = sat0.kepler.period();
        let ip_distance = sat0.distance(sat1);
        let ip_latency = ip_distance / SPEED_OF_LIGHT * 1000f64;
        let metrics = ShellMetrics {
            orbital_period,
            ip_distance,
            ip_latency,
        };

        let shell = Self {
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

    pub fn simulate(&mut self, ts: Timestamp) {
        self.orbits.iter_mut().for_each(|o| o.simulate(ts));
    }

    pub fn get_sat(&self, coords: ShellCoordinates) -> Option<&Satellite> {
        let (orbit, idx) = coords;
        self.orbits
            .get(orbit as usize)
            .and_then(|o| o.satellites.get(idx as usize))
    }

    pub fn get_sat_mut(&mut self, coords: ShellCoordinates) -> Option<&mut Satellite> {
        let (orbit, idx) = coords;
        self.orbits
            .get_mut(orbit as usize)
            .and_then(|o| o.satellites.get_mut(idx as usize))
    }

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

    pub fn get_cross_plane_latencies(&self) -> CrossPlaneLatencies {
        self.get_cross_plane_distances()
            .iter()
            .map(|(&(s1, s2), dist)| ((s1, s2), dist / SPEED_OF_LIGHT * 1000f64))
            .collect()
    }
}
