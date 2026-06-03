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

use crate::{ShellCoordinates, Timestamp, gateway::Gateway, walker::WalkerShellVariant};
use constellation_code::{Code, ConstellationCode};

#[derive(Debug, Clone)]
pub struct Constellation {
    pub cfg: ConstellationCode,
    pub shells: Vec<WalkerShellVariant>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectivityEventType {
    Addition,
    Removal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectivityEvent {
    pub sat: ShellCoordinates,
    pub gw: String,
    pub shell: usize,
    pub ts: Timestamp,
    pub event: ConnectivityEventType,
    pub distance: f64,
}

impl Constellation {
    pub fn gateways(&self) -> Vec<Gateway> {
        self.cfg.gateways.iter().map(|gw| gw.into()).collect()
    }

    pub fn get_gateway(&self, gw: String) -> Option<Gateway> {
        self.cfg
            .gateways
            .iter()
            .find(|current| current.name == gw)
            .and_then(|gw| Some(Gateway::from(gw)))
    }

    pub fn simulate(&mut self, ts: Timestamp) {
        self.shells.iter_mut().for_each(|shell| shell.simulate(ts));
    }

    pub fn orbital_periods(&self) -> Vec<(usize, f64)> {
        self.shells
            .iter()
            .enumerate()
            .map(|(idx, shell)| (idx, shell.orbital_period()))
            .collect()
    }

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

#[derive(Serialize, Deserialize)]
pub struct ConnectivityEventsDump {
    pub period: f64,
    pub events: HashMap<String, Vec<ConnectivityEvent>>,
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
