use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use tokio::{
    runtime,
    task::{self, JoinHandle},
};
use tqdm::pbar;

use crate::{
    ShellCoordinates, Timestamp,
    code::{Code, ConstellationCode},
    gateway::Gateway,
    walker::{WalkerShell, WalkerShellVariant, pattern::WalkerDelta},
};

#[derive(Debug)]
pub struct Constellation {
    cfg: ConstellationCode,
    pub shells: Vec<WalkerShellVariant>,
}

#[derive(Debug)]
pub enum ConnectivityEventType {
    Addition,
    Removal,
}

#[derive(Debug)]
pub struct ConnectivityEvent {
    pub sat: ShellCoordinates,
    pub gw: String,
    pub shell: usize,
    pub ts: Timestamp,
    pub event: ConnectivityEventType,
}

impl Constellation {
    pub fn gateways(&self) -> &Vec<Gateway> {
        &self.cfg.gateways
    }

    pub fn get_gateway(&self, gw: String) -> Option<&Gateway> {
        self.cfg.gateways.iter().find(|current| current.name == gw)
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
    ) -> HashMap<String, Vec<ConnectivityEvent>> {
        fn compute(
            old: HashSet<ShellCoordinates>,
            gw: Box<Gateway>,
            shell: Box<WalkerShell<WalkerDelta>>,
            idx: usize,
            ts: u64,
        ) -> (
            HashSet<ShellCoordinates>,
            String,
            Option<Vec<ConnectivityEvent>>,
        ) {
            let (coords, _dists) = gw
                .sats_in_sight(&shell.shell, ts as f64)
                .iter()
                .map(|(coord, dist)| (coord, dist))
                .collect::<(Vec<ShellCoordinates>, Vec<f64>)>();
            let current: HashSet<ShellCoordinates> = HashSet::from_iter(coords);
            if old == current {
                return (current, gw.name.clone(), None);
            }
            let lost = old.difference(&current);
            let new = current.difference(&old);

            let mut events = lost
                .into_iter()
                .map(|coord| ConnectivityEvent {
                    sat: *coord,
                    gw: gw.name.clone(),
                    shell: idx,
                    ts: ts as f64,
                    event: ConnectivityEventType::Removal,
                })
                .collect::<Vec<ConnectivityEvent>>();
            events.extend(new.into_iter().map(|coord| ConnectivityEvent {
                sat: *coord,
                gw: gw.name.clone(),
                shell: idx,
                ts: ts as f64,
                event: ConnectivityEventType::Addition,
            }));
            (current, gw.name.clone(), Some(events))
        }

        let mut ts = 0;
        let mut all_events: HashMap<String, Vec<ConnectivityEvent>> = Default::default();
        let mut bar = pbar(Some(duration as usize));
        let rt = runtime::Runtime::new().unwrap();
        rt.block_on(async {
            while ts < duration {
                self.simulate(step);
                ts += step as u64;

                for (idx, shell) in self.shells.iter().enumerate() {
                    let mut old_contacts: HashMap<String, HashSet<ShellCoordinates>> =
                        Default::default();
                    if let WalkerShellVariant::Delta(shell) = shell {
                        let handles: Vec<
                            JoinHandle<(
                                HashSet<ShellCoordinates>,
                                String,
                                Option<Vec<ConnectivityEvent>>,
                            )>,
                        > = self
                            .cfg
                            .gateways
                            .iter()
                            .map(|gw| {
                                let old = old_contacts.entry(gw.name.clone()).or_default();
                                let ts = ts.clone();
                                let idx = idx.clone();
                                let old_clone = old.clone();
                                let gw_clone = Box::new(gw.clone());
                                let shell = Box::new(shell.clone());

                                task::spawn(
                                    async move { compute(old_clone, gw_clone, shell, idx, ts) },
                                )
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
                }
                let _ = bar.update(step as usize);
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
