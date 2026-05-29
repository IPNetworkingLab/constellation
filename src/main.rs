use std::path::PathBuf;

use clap::Parser;
use tracing::{debug, info};

use constellation::{SPEED_OF_LIGHT, constellation::Constellation, walker::WalkerShellVariant};

#[derive(Parser)]
pub struct Cli {
    #[arg(long)]
    cfg: Option<PathBuf>,
    #[arg(long)]
    code: Option<String>,
    #[arg(long)]
    ts: Option<f64>,
    #[arg(long)]
    gw: Option<String>,
    #[arg(long)]
    shell: Option<usize>,
    // #[argc]
}

fn main() -> Result<(), ()> {
    tracing_subscriber::fmt::init();
    // satkit::utils::update_datafiles(None, false);

    let cli = Cli::parse();

    if let Some(cfg) = cli.cfg {
        info!("Parsing configuration file.");
        let mut constellation = Constellation::try_from(&cfg)?;
        if let Some(ts) = cli.ts {
            constellation.simulate(ts);
        }
        let periods = constellation.orbital_periods();
        let period = periods[0].1;
        info!(?period);
        let events = constellation.connectivity_events(period as u64, 1f64);
        info!(?events);
        // debug!(?constellation);
        // if let Some(ts) = cli.ts
        //     && let Some(gw) = cli.gw
        //     && let Some(gw) = constellation.get_gateway(gw)
        //     && let Some(shellidx) = cli.shell
        //     && let Some(shell) = constellation.shells.get(shellidx)
        // {
        //     info!(pos=?gw.position().to_geodetic_deg());
        //     if let WalkerShellVariant::Delta(shell) = shell {
        //         // let s = shell.shell.get_sat((1, 0)).unwrap();
        //         // info!(kep =?s.kepler);
        //         // let coord = s.itrf_coord(0f64).unwrap();
        //         // // info!(?s, irtf=?coord.to_geodetic_deg(), i2=?coord.to_geodetic_rad(), ?coord);
        //         // info!(sat=?coord, gw=?gw.position());
        //         // // let ned = coord.to_ned(&gw.position());
        //         // // info!(?ned);

        //         let in_sight = gw.sats_in_sight(&shell.shell, ts);
        //         for (coords, dist) in &in_sight {
        //             if let Some(sat) = shell.shell.get_sat(*coords) {
        //                 info!(coord=?sat.coord(), dist=?dist / SPEED_OF_LIGHT *1000f64);
        //             }
        //         }
        //         // println!("{in_sight:#?}");
        //     }

        //     // let in_sight = match shell {
        //     //     WalkerShellVariant::Star(shell) => gw.sats_in_sight(&shell.shell, ts),
        //     //     WalkerShellVariant::Delta(shell) => gw.sats_in_sight(&shell.shell, ts),
        //     // };
        // }
    } else if let Some(_code) = cli.code {
        unimplemented!()
    }

    Ok(())
}
