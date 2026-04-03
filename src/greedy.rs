use crate::covering_design::CoveringDesign;

pub fn greedy(cd: &CoveringDesign) -> Vec<usize> {
    let mut solution = Vec::<usize>::new();
    let covered_indices = cd
        .candidates
        .iter()
        .map(|&block| cd.get_covered_indices(block))
        .collect::<Vec<Vec<usize>>>();
    // bool陣列
    let mut uncovered = std::iter::repeat(true)
        .take(cd.m_subsets.len())
        .collect::<Vec<bool>>();
    let mut uncovered_count = cd.m_subsets.len();

    while uncovered_count > 0 {
        let block_i = (0..cd.candidates.len())
            .max_by_key(|&i| covered_indices[i].iter().filter(|&&j| uncovered[j]).count())
            .unwrap();
        solution.push(block_i);
        for &i in &covered_indices[block_i] {
            if uncovered[i] {
                uncovered[i] = false;
                uncovered_count -= 1;
            }
        }
    }

    solution
}
