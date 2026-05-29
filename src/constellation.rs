use std::path::PathBuf;

use crate::{
    Timestamp,
    code::{Code, ConstellationCode},
    gateway::Gateway,
    walker::WalkerShellVariant,
};

#[derive(Debug)]
pub struct Constellation {
    cfg: ConstellationCode,
    pub shells: Vec<WalkerShellVariant>,
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

    pub fn connectivity_events(&mut self, duration: Timestamp, step: f64) {}
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
