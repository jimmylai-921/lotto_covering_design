pub mod covering_design;
pub mod greedy;
pub mod simulated_annealing;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::covering_design::{CoveringDesign, Optimizer};
use crate::greedy::greedy;
use crate::simulated_annealing::{SimulatedAnnealing, SimulatedAnnealingArgs};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    v: u32,
    k: u32,
    m: u32,
    t: u32,
    #[arg(long)]
    load_path: Option<PathBuf>,
    #[arg(long)]
    save_path: Option<PathBuf>,
    #[arg(long)]
    save_invalid_path: Option<PathBuf>,
    #[arg(long)]
    path: Option<PathBuf>,
    #[arg(long, default_value_t = 1)]
    threads: usize,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(visible_alias("sa"))]
    SimulatedAnnealing(SimulatedAnnealingArgs),
}

pub static TERMINATE: AtomicBool = AtomicBool::new(false);

fn main() {
    let mut args = Args::parse();

    ctrlc::set_handler(|| TERMINATE.store(true, Ordering::Relaxed)).unwrap();

    args.load_path = args.load_path.or(args.path.clone());
    args.save_path = args.save_path.or(args.path.clone());
    args.save_invalid_path = args
        .save_invalid_path
        .or(args.save_path.as_ref().map(|save_path| {
            change_file_name(save_path, {
                let mut file_stem = save_path.file_stem().unwrap().to_owned();
                file_stem.push("-invalid");
                file_stem
            })
        }));

    let start = std::time::Instant::now();
    let cd = CoveringDesign::new(args.v, args.k, args.m, args.t);

    let mut solution: Option<Vec<usize>> = None;
    let mut invalid_solution: Option<Vec<usize>> = None;
    if let Some(load_path) = args.load_path {
        if let Some(loaded_solution) = cd.load_solution(&load_path) {
            if cd.is_solution_valid(&loaded_solution) {
                solution = Some(loaded_solution);
            } else {
                invalid_solution = Some(loaded_solution);
            }
        } else {
            solution = Some(greedy(&cd));
        }
    } else {
        solution = Some(greedy(&cd));
    }

    if let Some(ref invalid_solution) = invalid_solution {
        println!("初始注數(invalid): {}", invalid_solution.len());
    } else {
        println!("初始注數: {}", solution.as_ref().unwrap().len());
    }

    if let Some(command) = args.command {
        let optimizer: SimulatedAnnealing = match command {
            Commands::SimulatedAnnealing(sa_args) => SimulatedAnnealing::new(&cd, sa_args),
        };

        if let Some(ref invalid_solution2) = invalid_solution {
            let new_solution = optimizer.solve(&cd, &invalid_solution2, args.threads);
            if let Some(new_solution) = new_solution {
                if cd.is_solution_valid(&new_solution) {
                    solution = Some(new_solution);
                    invalid_solution = None;
                } else {
                    invalid_solution = Some(new_solution);
                }
            } else {
                invalid_solution = None;
            }
        } else {
            let (new_solution, new_invalid_solution) =
                optimizer.shrink(&cd, &solution.unwrap(), args.threads);
            solution = new_solution;
            invalid_solution = Some(new_invalid_solution);
        }
    }
    println!("時間: {}ms", start.elapsed().as_millis());

    if let Some(solution) = solution {
        println!("注數: {}", solution.len());
        cd.print_solution(&solution);
        if let Some(save_path) = args.save_path {
            cd.save_solution(&save_path, &solution);
        }
    }
    if let Some(invalid_solution) = invalid_solution {
        println!("注數(invalid): {}", invalid_solution.len());
        cd.print_solution(&invalid_solution);
        if let Some(save_invalid_path) = args.save_invalid_path {
            cd.save_solution(&save_invalid_path, &invalid_solution);
        }
    }
}

fn change_file_name(path: &Path, name: std::ffi::OsString) -> PathBuf {
    let mut result = path.to_owned();
    result.set_file_name(name);
    if let Some(ext) = path.extension() {
        result.set_extension(ext);
    }
    result
}
