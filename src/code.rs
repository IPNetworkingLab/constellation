use std::{collections::HashMap, path::PathBuf};

use clap::builder::OsStr;
use serde::Deserialize;

use crate::gateway::Gateway;

#[derive(Debug, Deserialize)]
pub enum Walker {
    Delta,
    Star,
}

pub enum WalkerError {
    Unsupported,
}

impl TryFrom<&str> for Walker {
    type Error = WalkerError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == "D" {
            Ok(Walker::Delta)
        } else if value == "S" {
            Ok(Walker::Star)
        } else {
            Err(WalkerError::Unsupported)
        }
    }
}

pub enum CodeError {
    CodeMalformed,
    Walker(WalkerError),
    AltitudeUnparsable,
    InclinationUnparsable,
    PlaneParameters(PlaneParametersError),
}

#[derive(Debug, Deserialize)]
pub struct Code {
    pub walker: Walker,
    pub altitude: f64,
    pub inclination: f64,
    pub plane_parameters: PlaneParameters,
}

#[derive(Debug, Deserialize)]
pub struct PlaneParameters {
    pub sats: u32,
    pub planes: u32,
    pub phasing: u32,
}

pub enum PlaneParametersError {
    Malformed,
    SatsUnparsable,
    PlanesUnparsable,
    PhasingFactorUnparsable,
}

impl TryFrom<&str> for PlaneParameters {
    type Error = PlaneParametersError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() != 3 {
            return Err(PlaneParametersError::Malformed);
        }

        let sats = str::parse::<u32>(parts[0]).map_err(|_| PlaneParametersError::SatsUnparsable)?;
        let planes =
            str::parse::<u32>(parts[1]).map_err(|_| PlaneParametersError::PlanesUnparsable)?;
        let phasing = str::parse::<u32>(parts[2])
            .map_err(|_| PlaneParametersError::PhasingFactorUnparsable)?;

        Ok(PlaneParameters {
            sats,
            planes,
            phasing,
        })
    }
}

impl TryFrom<String> for Code {
    type Error = CodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() < 4 {
            return Err(CodeError::CodeMalformed);
        }

        let walker = Walker::try_from(parts[0]).map_err(|err| CodeError::Walker(err))?;
        let altitude = str::parse::<f64>(parts[1]).map_err(|_| CodeError::AltitudeUnparsable)?;
        let inclination =
            str::parse::<f64>(parts[2]).map_err(|_| CodeError::InclinationUnparsable)?;
        let plane_parameters =
            PlaneParameters::try_from(parts[3]).map_err(|err| CodeError::PlaneParameters(err))?;
        Ok(Code {
            walker,
            altitude,
            inclination,
            plane_parameters,
        })
    }
}

impl TryFrom<&str> for Code {
    type Error = CodeError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Code::try_from(value.to_string())
    }
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
pub struct ConstellationShell {
    pub code: String,
    pub name: String,
    pub link_patterns: Vec<HashMap<String, usize>>,
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
pub struct ConstellationCode {
    pub version: String,
    pub shells: Vec<ConstellationShell>,
    pub gateways: Vec<Gateway>,
}

impl TryFrom<&PathBuf> for ConstellationCode {
    // FIXME
    type Error = ();
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        if !path.exists() || !path.is_file() || path.extension() != Some(&OsStr::from("yaml")) {
            return Err(());
        }
        let raw = std::fs::read(path).map_err(|_| ())?;
        yaml_serde::from_slice::<ConstellationCode>(&raw).map_err(|_| ())
    }
}
