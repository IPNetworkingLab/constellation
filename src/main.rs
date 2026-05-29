use clap::Parser;
use tracing::info;

use constellation::{
    SPEED_OF_LIGHT,
    code::{Code, Walker},
    shell::Neighbors,
    walker::{
        WalkerShell,
        pattern::{WalkerDelta, WalkerStar},
    },
};

#[derive(Parser)]
pub struct Cli {
    code: String,
}

fn main() -> Result<(), ()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let code = Code::try_from(cli.code).map_err(|_| ())?;
    match code.walker {
        Walker::Delta => {
            let mut shell = WalkerShell::<WalkerDelta>::try_from(&code)?;
            info!(?code, metrics=?shell.shell.metrics);
            for _i in 0..shell.shell.metrics.orbital_period as u64 {
                shell.shell.simulate(1f64);
                shell.shell.get_cross_plane_latencies();
            }
            // // info!(orbit0=?shell.shell.orbits[0]);
            // let sat0 = &shell.shell.orbits[0].satellites[0];
            // let sat1 = &shell.shell.orbits[0].satellites[1];
            // println!("{sat0:#?}\n{:#?}", sat0.position());
            // println!("{sat1:#?}\n{:#?}", sat1.position());

            // let sat0 = &shell.shell.orbits[0].satellites[0];
            // let sat1 = &shell.shell.orbits[1].satellites[0];
            // let n = WalkerDelta::neighbors(&shell.shell, (0, 0));
            // println!("{sat0:#?}\n{sat1:#?}\n{n:#?}");
            // let d = sat0.distance(sat1) / SPEED_OF_LIGHT * 1000f64;
            // println!("{d:#?}");

            // let dist = shell.shell.get_cross_plane_distances();
            // let d = dist.get(&((2, 8), (3, 8)));
            // // println!("{d:#?}");
            // let lat = shell.shell.get_cross_plane_latencies();
            // let l = lat.get(&((2, 8), (3, 8)));
            // // println!("{dist:#?}\n\n{lat:#?}");
            // println!("{d:#?}\n\n{l:#?}");

            //     shell.shell.simulate(60f64);
            //     let sat0 = &shell.shell.orbits[0].satellites[0];
            //     let sat1 = &shell.shell.orbits[0].satellites[1];
            //     println!("{sat0:#?}\n{sat1:#?}");
            // println!("{:#?}", shell.shell.orbits);
        }
        Walker::Star => {
            let shell = WalkerShell::<WalkerStar>::try_from(&code)?;
            info!(?shell);
        }
    };

    Ok(())
}
