use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::{
    runtime,
    task::{self, JoinHandle},
};
use tqdm::pbar;

// Import for docstrings.
#[allow(unused)]
use crate::shell::Shell;
use crate::{ShellCoordinates, Timestamp, gateway::Gateway, walker::WalkerShellVariant};
use constellation_code::{Code, ConstellationCode};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Types of connectivity events.
pub enum ConnectivityEventType {
    /// Addition of a link between two objects.
    Addition,
    /// Removal of a link between two objects.
    Removal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// A connectivity event between a satellite and a gateway.
pub struct ConnectivityEvent {
    /// [ShellCoordinates] of the target satellite.
    pub sat: ShellCoordinates,
    /// Name of the target [Gateway].
    pub gw: String,
    /// [Shell] index within the [Constellation].
    pub shell: usize,
    /// [Timestamp] at which the [ConnectivityEvent] happens.
    pub ts: Timestamp,
    /// The type of event.
    pub event: ConnectivityEventType,
    /// The distance between both objects (meters).
    pub distance: f64,
}

#[derive(Serialize, Deserialize)]
/// Structure representing the portable format for [`ConnectivityEvent`]
/// happening in the [`Constellation`].
pub struct ConnectivityEventsDump {
    /// Orbital period of the first [`Constellation`]'s [`Shell`].
    pub period: f64,
    /// Mapping between ground station name and the related [ConnectivityEvent]
    /// for the first [`Constellation`]'s [`Shell`].
    pub events: HashMap<String, Vec<ConnectivityEvent>>,
}

#[derive(Debug, Clone)]
/// Structure representing a satellite constellation.
pub struct Constellation {
    /// The constellation code, as defined by [Piraux and Fraire](https://datatracker.ietf.org/doc/draft-piraux-space-constellation-code/01/).
    pub cfg: ConstellationCode,
    /// List of [Shell] constituing the [Constellation].
    pub shells: Vec<WalkerShellVariant>,
}

impl Constellation {
    /// The list of [ground stations](Gateway) to which the [`Constellation`] can connect to.
    pub fn gateways(&self) -> Vec<Gateway> {
        self.cfg.gateways.iter().map(|gw| gw.into()).collect()
    }

    /// Get the [`Gateway`] identified by `gw` name, else [`None`] if the provided
    /// name does not correspond to an allowed ground station.
    pub fn get_gateway(&self, gw: String) -> Option<Gateway> {
        self.cfg
            .gateways
            .iter()
            .find(|current| current.name == gw)
            .and_then(|gw| Some(Gateway::from(gw)))
    }

    /// Determine the new position of all the satellite after `ts` seconds since
    /// the last timestamp.
    ///
    /// # Arguments
    ///
    /// - `ts` - The [`Timestamp`] at which the position is computed (seconds).
    pub fn simulate(&mut self, ts: Timestamp) {
        self.shells.iter_mut().for_each(|shell| shell.simulate(ts));
    }

    /// Return the orbital period for each [`Shell`] constituing the [`Constellation`].
    pub fn orbital_periods(&self) -> Vec<(usize, f64)> {
        self.shells
            .iter()
            .enumerate()
            .map(|(idx, shell)| (idx, shell.orbital_period()))
            .collect()
    }

    /// Compute the [`ConnectivityEvent`]s happening in the [`Constellation`] for
    /// `duration` with a time `step`.
    /// Each timestep is evaluated in a dedicated thread to speed up the computation.
    ///
    /// Optionally, dumps the events serialized according to the
    /// [`ConnectivityEventsDump`] format in a file located in `dump` directory.
    /// The provided path **must** follow the format `$ROOT/<variant>/<topo>`, where
    ///
    /// - `variant` corresponds to a user-defined experiment identifier.
    /// - `topo` corresponds to the [`Constellation`] name.
    ///
    /// This function dumps the [`ConnectivityEvent`]s in a file located at
    /// `$ROOT/kepler/<topo>.json`.
    ///
    /// # Arguments
    ///
    /// - `duration` - The simulated time duration (seconds)
    /// - `step` - The time step used to sample the `duration` (seconds).
    /// - `dump` - If specified, the directory to which the function dumps the
    /// computed [`ConnectivityEvent`]s.
    ///
    /// # Returns
    ///
    /// - The mapping between ground stations names and related [`ConnectivityEvent`]s.
    pub fn connectivity_events(
        &mut self,
        duration: u64,
        step: f64,
        dump: Option<PathBuf>,
    ) -> HashMap<String, Vec<ConnectivityEvent>> {
        fn compute(
            previous_in_sight: HashMap<ShellCoordinates, f64>,
            gw: Gateway,
            shell: Arc<Box<WalkerShellVariant>>,
            idx: usize,
            ts: u64,
            previous_ts: u64,
        ) -> (
            HashMap<ShellCoordinates, f64>,
            String,
            Option<Vec<ConnectivityEvent>>,
        ) {
            let in_sight: HashMap<(usize, usize), f64> =
                HashMap::from_iter(shell.sat_in_sight(&gw, ts as f64));
            let current: HashSet<ShellCoordinates> =
                HashSet::from_iter(in_sight.keys().into_iter().cloned());
            let old = HashSet::from_iter(previous_in_sight.keys().into_iter().cloned());
            if old == current {
                return (in_sight, gw.get_name().clone(), None);
            }
            let lost = old.difference(&current);
            let new = current.difference(&old);

            let mut events = lost
                .into_iter()
                .map(|coord| ConnectivityEvent {
                    sat: *coord,
                    gw: gw.get_name().clone(),
                    shell: idx,
                    ts: previous_ts as f64,
                    event: ConnectivityEventType::Removal,
                    distance: previous_in_sight[coord],
                })
                .collect::<Vec<ConnectivityEvent>>();
            events.extend(new.into_iter().map(|coord| ConnectivityEvent {
                sat: *coord,
                gw: gw.get_name().clone(),
                shell: idx,
                ts: ts as f64,
                event: ConnectivityEventType::Addition,
                distance: in_sight[coord],
            }));
            (in_sight, gw.get_name().clone(), Some(events))
        }

        let mut ts = 0;
        let mut all_events: HashMap<String, Vec<ConnectivityEvent>> = Default::default();
        let mut bar = pbar(Some(duration as usize));
        let rt = runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let mut old_contacts: HashMap<String, HashMap<ShellCoordinates, f64>> =
                Default::default();
            let gateways = self.gateways();
            while ts < duration {
                self.simulate(step);
                for (idx, shell) in self.shells.iter().enumerate() {
                    let shell = Arc::new(Box::new(shell.clone()));
                    let handles: Vec<
                        JoinHandle<(
                            HashMap<ShellCoordinates, f64>,
                            String,
                            Option<Vec<ConnectivityEvent>>,
                        )>,
                    > = gateways
                        .iter()
                        .map(|gw| {
                            let old = old_contacts
                                .entry(gw.get_name().clone())
                                .or_default()
                                .clone();
                            let ts = ts.clone();
                            let idx = idx.clone();
                            let gw_clone = gw.clone();
                            let shell = shell.clone();
                            task::spawn(async move {
                                compute(old, gw_clone, shell, idx, ts, ts - step as u64)
                            })
                        })
                        .collect();
                    for handle in handles {
                        let (current, gw, events) = handle.await.unwrap();
                        old_contacts.entry(gw.clone()).insert_entry(current);
                        if let Some(events) = events
                            && !events.is_empty()
                        {
                            all_events.entry(gw).or_default().extend(events);
                        }
                    }
                }
                ts += step as u64;
                let _ = bar.update(step as usize);
            }
            if let Some(path) = dump {
                // $ROOT/<variant>/<topo>/
                let topo = path.file_name().unwrap();
                let mut path = path.parent().unwrap().parent().unwrap().to_path_buf();
                path.push("kepler");
                if !path.exists() {
                    let _ = std::fs::create_dir(path.clone());
                }
                path.push(topo.to_str().unwrap());
                path.set_extension("json");
                let content = ConnectivityEventsDump {
                    period: self.orbital_periods()[0].1,
                    events: all_events.clone(),
                };
                let events = serde_json::to_string(&content).unwrap();
                let _ = std::fs::write(path, events);
            }

            all_events
        })
    }
}

impl TryFrom<&PathBuf> for Constellation {
    type Error = ();
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let cfg = ConstellationCode::try_from(path)?;
        let shells = cfg
            .shells
            .iter()
            .filter_map(|shell| {
                Code::try_from(shell.code.as_str())
                    .map_err(|_| ())
                    .and_then(|code| WalkerShellVariant::try_from(&code))
                    .ok()
            })
            .collect();
        Ok(Self { cfg, shells })
    }
}
