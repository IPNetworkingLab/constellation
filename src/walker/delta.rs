use crate::{
    ShellCoordinates,
    shell::{Neighbors, Pattern, Shell},
    walker::pattern::WalkerDelta,
};

impl Pattern for WalkerDelta {}

impl Neighbors<Self> for WalkerDelta {
    fn neighbors(shell: &Shell<WalkerDelta>, coords: ShellCoordinates) -> Vec<ShellCoordinates> {
        let (oidx, sidx) = coords;
        let ops = vec![1, -1];
        // In-plane connections.
        let mut neighbors: Vec<ShellCoordinates> = ops
            .iter()
            .map(|op| {
                (
                    oidx,
                    (((sidx as i64) + op).rem_euclid(shell.nsats_per_orbit as i64)) as usize,
                )
            })
            .collect();

        // Add cross-plane connections
        if shell.full_connect {
            let cp = vec![
                (
                    (oidx + 1).rem_euclid(shell.norbits),
                    if oidx as usize != shell.norbits - 1 {
                        sidx
                    } else {
                        (sidx + shell.phasing as usize).rem_euclid(shell.nsats_per_orbit)
                    },
                ),
                (
                    (oidx as i64 - 1).rem_euclid(shell.norbits as i64) as usize,
                    if oidx != 0 {
                        sidx
                    } else {
                        ((sidx as i64 - shell.phasing as i64)
                            .rem_euclid(shell.nsats_per_orbit as i64))
                            as usize
                    },
                ),
            ];
            neighbors.extend(cp);
        } else {
            // FIXME
            unimplemented!();
        }

        neighbors
    }
}
