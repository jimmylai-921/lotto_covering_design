use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::{BufRead, Write};
use std::path::Path;

pub struct Combinations {
    mask: u64,
    current: u64,
}

impl Combinations {
    pub fn new(n: u32, k: u32) -> Self {
        Self {
            mask: (1u64 << n) - 1,
            current: (1u64 << k) - 1,
        }
    }
}

// ref: https://andy.kitchen/combinations.html
impl Iterator for Combinations {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        let b = self.current;
        if b == 0 {
            return None;
        }
        let mask = self.mask;
        let u = b & (!b + 1);
        let mut v = u + b;
        v &= mask;
        if v == 0 {
            self.current = 0;
            return Some(b);
        }
        self.current = v + ((v ^ b) >> (u.trailing_zeros() + 2));
        Some(b)
    }
}

pub struct CoveringDesign {
    pub v: u32,
    pub k: u32,
    pub m: u32,
    pub t: u32,
    pub m_subsets: Vec<u64>,
    pub candidates: Vec<u64>,
}

impl CoveringDesign {
    pub fn new(v: u32, k: u32, m: u32, t: u32) -> Self {
        Self {
            v,
            k,
            m,
            t,
            m_subsets: Combinations::new(v, m).collect::<Vec<u64>>(),
            candidates: Combinations::new(v, k).collect::<Vec<u64>>(),
        }
    }

    pub fn get_covered_indices(&self, block: u64) -> Vec<usize> {
        self.m_subsets
            .iter()
            .enumerate()
            .filter(|(_i, &comb)| (comb & block).count_ones() >= self.t)
            .map(|(i, _comb)| i)
            .collect::<Vec<usize>>()
    }

    pub fn is_solution_valid(&self, solution: &[usize]) -> bool {
        self.m_subsets.iter().all(|comb| {
            solution
                .iter()
                .any(|&block_i| (comb & self.candidates[block_i]).count_ones() >= self.t)
        })
    }

    pub fn uncovered_count(&self, solution: &[usize]) -> usize {
        self.m_subsets
            .iter()
            .filter(|&comb| {
                !solution
                    .iter()
                    .any(|&i| (self.candidates[i] & comb).count_ones() >= self.t)
            })
            .count()
    }

    pub fn generate_block_to_index_map(&self) -> HashMap<u64, usize> {
        self.candidates
            .iter()
            .enumerate()
            .map(|(i, &block)| (block, i))
            .collect::<HashMap<u64, usize>>()
    }

    pub fn generate_neighbors(&self) -> Vec<Vec<usize>> {
        let mut neighbors = Vec::<Vec<usize>>::with_capacity(self.candidates.len());
        // reverse index mapping
        let block_to_index = self.generate_block_to_index_map();

        for &block in &self.candidates {
            let mut v = Vec::<usize>::with_capacity((self.k * (self.v - self.k)) as usize);
            // 0011 -> 1001, 0101, 1010, 0110
            // println!("{:#016b}", block);
            for remove_index in block.trailing_zeros()..self.v {
                if block & (1u64 << remove_index) == 0 {
                    continue;
                }
                for add_index in block.trailing_ones()..self.v {
                    if block & (1u64 << add_index) != 0 {
                        continue;
                    }
                    let new_block = (block & (!(1u64 << remove_index))) | (1u64 << add_index);
                    // assert_eq!(new_block.count_ones(), cd.k);
                    v.push(block_to_index[&new_block]);
                }
            }
            // assert_eq!(v.len(), (cd.k * (cd.v - cd.k)) as usize);
            neighbors.push(v);
        }
        neighbors
    }

    pub fn generate_covered_indices(&self) -> Vec<Vec<usize>> {
        self.candidates
            .iter()
            .map(|&block| self.get_covered_indices(block))
            .collect::<Vec<Vec<usize>>>()
    }

    pub fn get_numbers(&self, block_i: usize) -> Vec<u32> {
        (1..=u64::BITS)
            .filter(|i| (self.candidates[block_i] & (1u64 << (i - 1))) != 0)
            .collect::<Vec<u32>>()
    }

    pub fn numbers_to_comb(&self, numbers_str: &str) -> u64 {
        let mut comb = 0u64;
        for num_str in numbers_str.split(' ') {
            let num = num_str.parse::<u32>().unwrap() - 1;
            if num >= self.v {
                panic!("num >= v");
            }

            comb |= 1 << num;
        }

        if comb.count_ones() != self.k {
            panic!("num count != k");
        }

        comb
    }

    pub fn get_blocks(&self, solution: &[usize]) -> Vec<Vec<u32>> {
        let mut blocks = solution
            .iter()
            .map(|&block_i| self.get_numbers(block_i))
            .collect::<Vec<Vec<u32>>>();
        blocks.sort();
        blocks
    }

    pub fn print_solution(&self, solution: &[usize]) {
        for block in self.get_blocks(solution) {
            println!("{:?}", block);
        }
    }

    pub fn load_solution(&self, path: &Path) -> Option<Vec<usize>> {
        let file = fs::File::open(path).ok()?;
        let mut solution = Vec::<usize>::new();
        let block_to_index = self.generate_block_to_index_map();
        let reader = io::BufReader::new(file);
        for line in reader.lines().map(|l| l.unwrap()) {
            let comb = self.numbers_to_comb(&line);
            solution.push(block_to_index[&comb]);
        }
        Some(solution)
    }

    pub fn save_solution(&self, path: &Path, solution: &[usize]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = io::BufWriter::new(file);
        for block in self.get_blocks(solution) {
            let line = block
                .iter()
                .map(u32::to_string)
                .collect::<Vec<String>>()
                .join(" ");
            writeln!(writer, "{}", line).unwrap();
        }
        writer.flush().unwrap();
    }
}

pub trait Optimizer: Send + Sync {
    fn run_many_threads(
        &self,
        cd: &CoveringDesign,
        initial_solution: &[usize],
        thread_count: usize,
    ) -> Vec<usize> {
        let solutions = std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(thread_count);
            for _ in 0..thread_count {
                let initial_solution_clone = initial_solution.to_vec();
                let handle = scope.spawn(move || self.run(&cd, &initial_solution_clone));
                handles.push(handle);
            }
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<Vec<usize>>>()
        });
        // pick lowest cost solution
        solutions
            .into_iter()
            .min_by_key(|solution| cd.uncovered_count(solution))
            .unwrap()
    }

    fn run(&self, cd: &CoveringDesign, initial_solution: &[usize]) -> Vec<usize>;

    fn solve(
        &self,
        cd: &CoveringDesign,
        initial_solution: &[usize],
        thread_count: usize,
    ) -> Option<Vec<usize>> {
        let cost = cd.uncovered_count(&initial_solution);
        let new_solution = self.run_many_threads(&cd, &initial_solution, thread_count);
        let new_cost = cd.uncovered_count(&new_solution);
        return if new_cost < cost {
            println!("找到更好的解, cost {} -> {}", cost, new_cost);
            Some(new_solution)
        } else {
            println!("未找到更好的解");
            None
        };
    }

    fn shrink(
        &self,
        cd: &CoveringDesign,
        initial_solution: &[usize],
        thread_count: usize,
    ) -> (Option<Vec<usize>>, Vec<usize>) {
        let mut solution = initial_solution.to_vec();
        let mut best_valid_solution: Option<Vec<usize>> = None;
        loop {
            solution.pop();
            let new_solution = self.run_many_threads(&cd, &solution, thread_count);
            if cd.is_solution_valid(&new_solution) {
                println!("找到{}注的解", new_solution.len());
                best_valid_solution = Some(new_solution.clone());
                solution = new_solution;
            } else {
                return (best_valid_solution, new_solution);
            }
        }
    }
}
