use satkit::ITRFCoord;
use serde::Deserialize;

use crate::{
    ShellCoordinates, Timestamp,
    shell::{Pattern, Shell},
};

#[derive(Deserialize, Debug)]
pub struct Gateway {
    pub name: String,
    #[serde(alias = "latitude_deg")]
    latitude: f64,
    #[serde(alias = "longitude_deg")]
    longitude: f64,
    #[serde(alias = "altitude_m")]
    altitude: f64,
    #[serde(alias = "min_elevation_deg")]
    min_elevation: f64,
}

impl Gateway {
    pub fn position(&self) -> ITRFCoord {
        ITRFCoord::from_geodetic_deg(self.latitude, self.longitude, self.altitude)
    }

    // fn distance(&self, coord: ITRFCoord) -> Option<f64> {
    //     let gw_pos = self.position();
    // }

    pub fn sats_in_sight<P: Pattern>(
        &self,
        shell: &Shell<P>,
        ts: Timestamp,
    ) -> Vec<(ShellCoordinates, f64)> {
        let gw_pos = self.position();

        shell
            .orbits
            .iter()
            .map(|o| {
                o.satellites
                    .iter()
                    .filter_map(|s| {
                        // FIXME handle errors.
                        let sat_pos = s.itrf_coord(ts).unwrap();
                        let ned = sat_pos.to_ned(&gw_pos);
                        let norm = ned.norm();
                        let ned_hat = ned / norm;
                        let elevation = (-ned_hat[2]).asin().to_degrees();
                        (elevation >= self.min_elevation).then_some((s.coord(), norm))
                    })
                    .collect::<Vec<(ShellCoordinates, f64)>>()
            })
            .flatten()
            .collect()
    }
}
