use std::sync::Arc;

use satkit::ITRFCoord;

use constellation_code::Gateway as InnerGateway;

use crate::{
    SPEED_OF_LIGHT, ShellCoordinates, Timestamp,
    shell::{Pattern, Satellite, Shell},
};

#[derive(Debug, Clone)]
pub struct Gateway(Arc<InnerGateway>);

impl<'a> From<&'a InnerGateway> for Gateway {
    fn from(value: &'a InnerGateway) -> Self {
        Self(Arc::new(value.clone()))
    }
}

impl Gateway {
    pub fn get_name(&self) -> &String {
        &self.0.name
    }

    pub fn position(&self) -> ITRFCoord {
        ITRFCoord::from_geodetic_deg(self.0.latitude, self.0.longitude, self.0.altitude)
    }

    fn distance_to_sat(&self, s: &Satellite, ts: Timestamp) -> Option<f64> {
        let gw_pos = self.position();
        let sat_pos = s.itrf_coord(ts).unwrap();
        let ned = sat_pos.to_ned(&gw_pos);
        let norm = ned.norm();
        let ned_hat = ned / norm;
        let elevation = (-ned_hat[2]).asin().to_degrees();
        (elevation >= self.0.min_elevation).then_some(norm)
    }

    pub fn distance(&self, s: &Satellite, ts: Timestamp) -> Option<f64> {
        self.distance_to_sat(s, ts)
    }

    pub fn latency(&self, s: &Satellite, ts: Timestamp) -> Option<f64> {
        self.distance_to_sat(s, ts)
            .and_then(|dist| Some(dist / SPEED_OF_LIGHT * 1000f64))
    }

    pub fn sats_in_sight<P: Pattern>(
        &self,
        shell: &Shell<P>,
        ts: Timestamp,
    ) -> Vec<(ShellCoordinates, f64)> {
        shell
            .orbits
            .iter()
            .map(|o| {
                o.satellites
                    .iter()
                    .filter_map(|s| {
                        self.distance_to_sat(s, ts)
                            .and_then(|norm| Some((s.coord(), norm)))
                    })
                    .collect::<Vec<(ShellCoordinates, f64)>>()
            })
            .flatten()
            .collect()
    }
}
