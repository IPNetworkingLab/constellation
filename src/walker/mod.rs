pub mod delta;
pub mod star;

use crate::{
    Timestamp,
    code::{Code, Walker},
    shell::{Pattern, Shell},
    walker::pattern::{WalkerDelta, WalkerStar},
};

pub trait WalkerPattern
where
    Self: Pattern + Clone,
{
    const RAAN_SPAN: f64;
}

pub mod pattern {
    use super::WalkerPattern;
    #[derive(Debug, Clone)]
    pub struct WalkerDelta();
    impl WalkerPattern for WalkerDelta {
        const RAAN_SPAN: f64 = 360f64;
    }

    #[derive(Debug, Clone)]
    pub struct WalkerStar();
    impl WalkerPattern for WalkerStar {
        const RAAN_SPAN: f64 = 180f64;
    }
}

#[derive(Debug, Clone)]
pub struct WalkerShell<WalkerPattern> {
    pub shell: Shell<WalkerPattern>,
    // TODO: link_pattern
}

impl<P> WalkerShell<P>
where
    P: WalkerPattern,
{
    pub fn new(
        norbits: usize,
        nsats: usize,
        altitude: f64,
        inclination: f64,
        phasing: u64,
    ) -> Result<Self, ()> {
        let shell = Shell::new(norbits, nsats, altitude, inclination, phasing, P::RAAN_SPAN)?;
        Ok(Self { shell })
    }
}

#[derive(Debug)]
pub enum WalkerShellVariant {
    Star(WalkerShell<WalkerStar>),
    Delta(WalkerShell<WalkerDelta>),
}

impl WalkerShellVariant {
    pub fn simulate(&mut self, ts: Timestamp) {
        match self {
            Self::Star(shell) => shell.shell.simulate(ts),
            Self::Delta(shell) => shell.shell.simulate(ts),
        }
    }

    pub fn orbital_period(&self) -> f64 {
        match self {
            Self::Star(shell) => shell.shell.metrics.orbital_period,
            Self::Delta(shell) => shell.shell.metrics.orbital_period,
        }
    }
}

// impl WalkerShellVariant {
//     fn shell<P: WalkerPattern>(&self) -> Shell<WalkerPattern> {
//         match self {
//             Self::Star(shell) => shell.shell,
//             Self::Delta(delta) => delta.shell,
//         }
//     }
// }

impl TryFrom<&Code> for WalkerShellVariant {
    type Error = ();
    fn try_from(code: &Code) -> Result<Self, Self::Error> {
        let variant = match code.walker {
            Walker::Star => Self::Star(WalkerShell::<WalkerStar>::try_from(code)?),
            Walker::Delta => Self::Delta(WalkerShell::<WalkerDelta>::try_from(code)?),
        };
        Ok(variant)
    }
}

impl TryFrom<&Code> for WalkerShell<WalkerStar> {
    type Error = ();
    fn try_from(code: &Code) -> Result<Self, Self::Error> {
        if !matches!(code.walker, Walker::Star) {
            return Err(());
        }
        WalkerShell::<WalkerStar>::new(
            code.plane_parameters.planes as usize,
            code.plane_parameters.sats as usize,
            code.altitude * 1000f64,
            code.inclination,
            code.plane_parameters.phasing as u64,
        )
    }
}

impl TryFrom<&Code> for WalkerShell<WalkerDelta> {
    type Error = ();
    fn try_from(code: &Code) -> Result<Self, Self::Error> {
        if !matches!(code.walker, Walker::Delta) {
            return Err(());
        }
        WalkerShell::<WalkerDelta>::new(
            code.plane_parameters.planes as usize,
            code.plane_parameters.sats as usize,
            code.altitude * 1000f64,
            code.inclination,
            code.plane_parameters.phasing as u64,
        )
    }
}
