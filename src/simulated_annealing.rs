use rand::prelude::*;
use rand::Rng;
use std::sync::atomic::Ordering;

use crate::covering_design::{CoveringDesign, Optimizer};

#[derive(Clone, Copy, clap::Args)]
pub struct SimulatedAnnealingArgs {
    #[arg(long, default_value_t = 0.9995)]
    cooling_factor: f64,
    #[arg(long, default_value_t = 1.0)]
    iter_factor: f64,
    #[arg(long, default_value_t = 30.0)]
    initial_temp: f64,
    #[arg(long, default_value_t = 1e-3)]
    min_temp: f64,
}

pub struct SimulatedAnnealing {
    // neighbors[block_index] -> Vec<block_indices>
    neighbors: Vec<Vec<usize>>,
    covered_indices: Vec<Vec<usize>>,
    // rng: rand::rngs::SmallRng,
    args: SimulatedAnnealingArgs,
}

/*
const EXP_TABLE_SIZE: usize = 2000;
const MAX_EXPONENT: f64 = 15.0;
lazy_static! {
    static ref INV_EXP_TABLE: [f64; EXP_TABLE_SIZE] = {
        let mut table = [0.0; EXP_TABLE_SIZE];
        for i in 0..EXP_TABLE_SIZE {
            table[i] = (-(i as f64 / (EXP_TABLE_SIZE as f64) * MAX_EXPONENT)).exp();
        }
        table
    };
}
*/

impl SimulatedAnnealing {
    pub fn new(cd: &CoveringDesign, args: SimulatedAnnealingArgs) -> Self {
        Self {
            neighbors: cd.generate_neighbors(),
            covered_indices: cd.generate_covered_indices(),
            // rng: rand::rngs::SmallRng::from_os_rng(),
            args,
        }
    }

    #[inline(always)]
    fn accept_prob(cost_delta: i32, temp: f64) -> f64 {
        let exponent = (cost_delta as f64) / temp;
        (-exponent).exp()
    }
}

impl Optimizer for SimulatedAnnealing {
    fn run(&self, cd: &CoveringDesign, initial_solution: &[usize]) -> Vec<usize> {
        let mut rng = rand::rngs::SmallRng::from_os_rng();
        let mut solution = initial_solution.to_vec();
        let mut best_solution = solution.clone();
        let mut cost = cd.uncovered_count(&solution) as i32;
        let mut best_cost = cost;
        let mut covered_counts = cd
            .m_subsets
            .iter()
            .map(|&comb| {
                solution
                    .iter()
                    .filter(|&&i| (cd.candidates[i] & comb).count_ones() >= cd.t)
                    .count()
            })
            .collect::<Vec<usize>>();
        /* let mut temp = self.initial_temp(&solution, &covered_counts); */
        let mut temp = self.args.initial_temp;
        let iter_count =
            (solution.len() as f64 * cd.k as f64 * (cd.v - cd.k) as f64 * self.args.iter_factor)
                as usize;
        let mut prev_time = std::time::Instant::now();
        let mut iters_since_print = 0;
        loop {
            for _ in 0..iter_count {
                unsafe {
                    let mut cost_delta = 0i32;
                    let change_index = rng.random_range(1..solution.len());
                    let change_from = *solution.get_unchecked(change_index);
                    let change_to = *self
                        .neighbors
                        .get_unchecked(change_from)
                        .choose(&mut rng)
                        .unwrap();
                    let removed_covers = self.covered_indices.get_unchecked(change_from);
                    let new_covers = self.covered_indices.get_unchecked(change_to);

                    for &rem_i in removed_covers {
                        let count = covered_counts.get_unchecked_mut(rem_i);
                        if *count == 1 {
                            cost_delta += 1;
                        }
                        *count -= 1;
                    }

                    for &new_i in new_covers {
                        if *covered_counts.get_unchecked(new_i) == 0 {
                            cost_delta -= 1;
                        }
                    }

                    if cost_delta <= 0 || rng.random::<f64>() < Self::accept_prob(cost_delta, temp)
                    {
                        // accept
                        cost += cost_delta;
                        *solution.get_unchecked_mut(change_index) = change_to;
                        if cost == 0 {
                            return solution;
                        }
                        if cost < best_cost {
                            best_cost = cost;
                            best_solution = solution.clone();
                        }
                        for &new_i in new_covers {
                            *covered_counts.get_unchecked_mut(new_i) += 1;
                        }
                    } else {
                        // revert
                        for &rem_i in removed_covers {
                            *covered_counts.get_unchecked_mut(rem_i) += 1;
                        }
                    }
                }
            }
            iters_since_print += 1;
            temp *= self.args.cooling_factor;
            let elapsed_secs = prev_time.elapsed().as_secs_f64();
            if elapsed_secs > 3.0 {
                println!(
                    "{:.0} iters/s, temp {}, cost {}, best_cost {}",
                    ((iters_since_print * iter_count) as f64) / elapsed_secs,
                    temp,
                    cost,
                    best_cost
                );
                iters_since_print = 0;
                prev_time = std::time::Instant::now();
            }
            if temp < self.args.min_temp || crate::TERMINATE.load(Ordering::Relaxed) {
                return best_solution;
            }
        }
    }
}
