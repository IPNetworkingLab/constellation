pub mod delta;
pub mod star;

use constellation_code::{
    Code,
    code::PlaneParameters,
    patterns::{Pattern, WalkerPattern as Walker},
};

use crate::{
    CrossPlaneLatencies, ShellCoordinates, Timestamp,
    shell::{Pattern as ShellPattern, Satellite, Shell},
    walker::pattern::{WalkerDelta, WalkerStar},
};

pub trait WalkerPattern
where
    Self: ShellPattern + Clone,
{
    const RAAN_SPAN: f64;
    const PATTERN: Pattern;
}

pub mod pattern {
    use constellation_code::patterns::Pattern;

    use super::WalkerPattern;
    #[derive(Debug, Clone)]
    pub struct WalkerDelta();
    impl WalkerPattern for WalkerDelta {
        const RAAN_SPAN: f64 = 360f64;
        const PATTERN: Pattern =
            Pattern::Walker(constellation_code::patterns::WalkerPattern::Delta);
    }

    #[derive(Debug, Clone)]
    pub struct WalkerStar();
    impl WalkerPattern for WalkerStar {
        const RAAN_SPAN: f64 = 180f64;
        const PATTERN: Pattern = Pattern::Walker(constellation_code::patterns::WalkerPattern::Star);
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
        // FIXME !!! impl From<Code> for Shell ...
        let code = Code {
            pattern: P::PATTERN,
            altitude,
            inclination,
            plane_parameters: PlaneParameters {
                sats: nsats as u32,
                planes: norbits as u32,
                phasing: phasing as u32,
            },
        };
        let shell = Shell::new(
            code,
            norbits,
            nsats,
            altitude,
            inclination,
            phasing,
            P::RAAN_SPAN,
        )?;
        Ok(Self { shell })
    }
}

#[derive(Debug, Clone)]
pub enum WalkerShellVariant {
    Star(WalkerShell<WalkerStar>),
    Delta(WalkerShell<WalkerDelta>),
}

impl WalkerShellVariant {
    pub fn get_sat(&self, coords: ShellCoordinates) -> Option<&Satellite> {
        match self {
            Self::Star(shell) => shell.shell.get_sat(coords),
            Self::Delta(shell) => shell.shell.get_sat(coords),
        }
    }

    pub fn simulate(&mut self, ts: Timestamp) {
        match self {
            Self::Star(shell) => shell.shell.simulate(ts),
            Self::Delta(shell) => shell.shell.simulate(ts),
        }
    }

    pub fn code(&self) -> &Code {
        match self {
            Self::Star(shell) => &shell.shell.code,
            Self::Delta(shell) => &shell.shell.code,
        }
    }

    pub fn orbital_period(&self) -> f64 {
        match self {
            Self::Star(shell) => shell.shell.metrics.orbital_period,
            Self::Delta(shell) => shell.shell.metrics.orbital_period,
        }
    }

    pub fn get_cross_plane_latencies(&self) -> CrossPlaneLatencies {
        match self {
            Self::Star(shell) => shell.shell.get_cross_plane_latencies(),
            Self::Delta(shell) => shell.shell.get_cross_plane_latencies(),
        }
    }

    pub fn get_in_plane_latency(&self) -> f64 {
        match self {
            Self::Star(shell) => shell.shell.metrics.ip_latency,
            Self::Delta(shell) => shell.shell.metrics.ip_latency,
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
        // TODO: Remove when more patterns added.
        #[allow(irrefutable_let_patterns)]
        if let Pattern::Walker(pattern) = &code.pattern {
            let variant = match pattern {
                Walker::Star => Self::Star(WalkerShell::<WalkerStar>::try_from(code)?),
                Walker::Delta => Self::Delta(WalkerShell::<WalkerDelta>::try_from(code)?),
            };
            Ok(variant)
        } else {
            Err(())
        }
    }
}

impl TryFrom<&Code> for WalkerShell<WalkerStar> {
    type Error = ();
    fn try_from(code: &Code) -> Result<Self, Self::Error> {
        if !matches!(code.pattern, Pattern::Walker(Walker::Star)) {
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
        if !matches!(code.pattern, Pattern::Walker(Walker::Delta)) {
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
