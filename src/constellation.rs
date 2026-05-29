use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use tqdm::pbar;

use crate::{
    ShellCoordinates, Timestamp,
    code::{Code, ConstellationCode},
    gateway::Gateway,
    walker::WalkerShellVariant,
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
        let mut ts = 0;
        let mut all_events: HashMap<String, Vec<ConnectivityEvent>> = Default::default();
        let mut bar = pbar(Some(duration as usize));
        while ts < duration {
            self.simulate(step);
            ts += step as u64;

            self.shells.iter().enumerate().for_each(|(idx, shell)| {
                let mut old_contacts: HashMap<&String, HashSet<ShellCoordinates>> =
                    Default::default();
                for gw in &self.cfg.gateways {
                    if let WalkerShellVariant::Delta(shell) = shell {
                        let (coords, _dists) = gw
                            .sats_in_sight(&shell.shell, ts as f64)
                            .iter()
                            .map(|(coord, dist)| (coord, dist))
                            .collect::<(Vec<ShellCoordinates>, Vec<f64>)>();
                        let current_contacts: HashSet<ShellCoordinates> =
                            HashSet::from_iter(coords);
                        let old = old_contacts
                            .entry(&gw.name)
                            .or_insert(current_contacts.clone());
                        if old == &current_contacts {
                            continue;
                        }
                        let lost = old.difference(&current_contacts);
                        let new = current_contacts.difference(&old);

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

                        *old = current_contacts;
                        if !events.is_empty() {
                            all_events
                                .entry(gw.name.clone())
                                .or_default()
                                .extend(events);
                        }
                    }
                }
            });
            let _ = bar.update(step as usize);
            // info!(?ts, ?duration, diff = ts < duration);
        }
        all_events
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
