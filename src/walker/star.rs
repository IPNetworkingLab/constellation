use crate::{
    ShellCoordinates,
    shell::{Neighbors, Pattern, Shell},
    walker::pattern::WalkerStar,
};

impl Pattern for WalkerStar {}

impl Neighbors<Self> for WalkerStar {
    fn neighbors(shell: &Shell<WalkerStar>, coords: ShellCoordinates) -> Vec<ShellCoordinates> {
        fn op_to_coord(op: i64, norbits: usize, oidx: usize, sidx: usize) -> ShellCoordinates {
            (
                (((oidx as i64) + op).rem_euclid(norbits as i64)) as usize,
                sidx,
            )
        }
        let (oidx, sidx) = coords;
        let ops = vec![-1, 1];
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
        if oidx != 0 {
            neighbors.push(op_to_coord(-1, shell.norbits, oidx, sidx));
        }

        if oidx as usize != shell.norbits - 1 {
            neighbors.push(op_to_coord(1, shell.norbits, oidx, sidx));
        }

        neighbors
    }
}
