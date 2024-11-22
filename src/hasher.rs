use colored::Colorize;
use primitive_types::U256;
use rand::Rng;
use rand_mt::Mt19937GenRand64;
use sha2::{Digest, Sha256};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_mm_prefetch;
use std::{
    iter,
    time::{Duration, Instant},
};

use crate::{meets_target, verify};
use rayon::{option, prelude::*};
use std::sync::Arc;

const CHUNK_SIZE: usize = 1024; 
const GRAPH_SIZE: usize = 2008;
const MAX_CHECKS: usize = 200000;
const PACKED_GRAPH_SIZE: usize = ((GRAPH_SIZE * GRAPH_SIZE / 2) + 31) / 32;

#[derive(Debug)]
struct GraphData {
    d: [u32; PACKED_GRAPH_SIZE],
    _size: usize,
}

impl GraphData {
    fn new(size: usize) -> Self {
        GraphData {
            d: [0; PACKED_GRAPH_SIZE],
            _size: size,
        }
    }

    fn coord_to_idx(&self, i: usize, j: usize) -> usize {
        if i <= j {
            i * self._size - ((i * (i + 1)) >> 1) + (j - (i + 1))
        } else {
            j * self._size - ((j * (j + 1)) >> 1) + (i - (j + 1))
        }
    }

    fn get(&self, i: usize, j: usize) -> bool {
        let idx = self.coord_to_idx(i, j);
        let bit_pos = idx & 0x1f;
        let val = self.d[idx >> 5] & (1 << (31 - bit_pos));
        val != 0
    }

    fn size(&self) -> usize {
        self._size
    }
}


fn generate_graph_opt(hash: &[u8; 32], grid_size: usize) -> GraphData {
    let mut graph = GraphData::new(grid_size);

    // Extract seed from hash
    let seed = extract_seed_from_hash(hash);

    // Initialize PRNG with seed
    let mut prng = Mt19937GenRand64::from(seed.to_le_bytes());

    // Fill the adjacency matrix
    for dw in 0..PACKED_GRAPH_SIZE {
        graph.d[dw] = prng.next_u32();
    }

    graph
}

fn hamiltonian_cycle_util(
    graph: &GraphData,
    path: &mut Vec<u16>,
    pos: usize,
    freev: &mut Vec<u16>,
    nchecks: &mut usize,
    maxchecks: usize,
) -> bool {
    *nchecks += 1;
    if *nchecks > maxchecks{ // MAX_CHECKS {
        return false;
    }

    if pos == graph.size() {
        if graph.get(path[pos - 1] as usize, path[0] as usize) {
            return true;
        } else {
            return false;
        }
    }

    let mut prev_pos = 0;

    loop {
        let v = freev[prev_pos];
        if v as usize == graph.size() {
            break;
        }

        if graph.get(path[pos - 1] as usize, v as usize) {
            path[pos] = v;
            freev[prev_pos] = freev[v as usize];

            if hamiltonian_cycle_util(graph, path, pos + 1, freev, nchecks, maxchecks) {
                return true;
            }
            if *nchecks > MAX_CHECKS {
                return false;
            }

            freev[prev_pos] = v;
            path[pos] = u16::MAX;
        }

        prev_pos = v as usize;
    }

    false
}

#[inline]
fn read_le_u64(bytes: &[u8]) -> u64 {
    let arr: [u8; 8] = bytes[..8].try_into().expect("Slice with incorrect length");
    u64::from_le_bytes(arr)
}
#[inline]
fn get_u64(data: &[u8], pos: usize) -> u64 {
    read_le_u64(&data[pos * 8..(pos + 1) * 8])
}
#[inline]
fn extract_seed_from_hash(hash: &[u8]) -> u64 {
    get_u64(hash, 0)
}

fn get_grid_size(hash: &[u8]) -> u16 {
    let grid_size_segment = &hash[28..32];
    //println!("byte grid_size_segment = {:?}", grid_size_segment);
    let grid_size: u64 = u32::from_le_bytes(
        grid_size_segment
            .try_into()
            .expect("Slice with incorrect length"),
    ) as u64;
    //println!("byte grid_size = {:?}", grid_size);
    let min_grid_size = 2000u64;
    let max_grid_size = GRAPH_SIZE as u64; 

    let mut grid_size_final = min_grid_size + (grid_size % (max_grid_size - min_grid_size));
    if grid_size_final > max_grid_size {
        grid_size_final = max_grid_size;
    }
    grid_size_final as u16
}


#[inline]
pub fn generate_nonce() -> [u8; 4] {
    let mut rng = rand::thread_rng();
    let nonce: u32 = rng.gen();
    nonce.to_be_bytes()
}

pub fn generate_nonce_and_find_cycle(target: &[u8],data: &[u8], maxchecks:usize, batchsize:usize) -> ([u8; 4], /* [u8; 32],*/ Vec<(String, String)>, Vec<Vec<u16>>,bool) {
    let nonce_bytes = generate_nonce();
    let mut cntn = 0;
    let mut pairs: Vec<(String, String)> = Vec::new();
    //let mut vdf_solution = vec![0xFFu8; GRAPH_SIZE * 2]; // 0xFFFF is 2 bytes, hence * 2

    let mut data_with_vdf = [0xFFu8; 4096];
    data_with_vdf[..76].copy_from_slice(data);
    data_with_vdf[76..80].copy_from_slice(&nonce_bytes);

    let hash1 = Sha256::digest(&data_with_vdf);

    let grid_size = get_grid_size(&hash1);
    let hash_ref: [u8; 32] = hash1.clone().into();
    let mut nchecks = 0;
    let graph = generate_graph_opt(&hash_ref, grid_size as usize);

    let mut freev = (1..=graph.size() as u16).collect::<Vec<u16>>();
    let mut path = vec![u16::MAX; grid_size as usize];
    path[0] = 0;

    if !hamiltonian_cycle_util(&graph, &mut path, 1, &mut freev, &mut nchecks,maxchecks) {
        return ([0; 4], Vec::new(),Vec::new(), false);
    }
    let mut similar_path = Vec::<Vec<u16>>::new();
    similar_path.push(path.clone());
    'outdoor: for dis in 1..path.len() - 2 {
        for start in 1..path.len() - 1 - dis {
            let s = start;
            let e = s + dis;
            let mut path_vec = path.clone();
            if graph.get(path[s - 1] as usize, path[e] as usize) == true
                && graph.get(path[s] as usize, path[e + 1] as usize) == true
            {
                path_vec[s..=e].reverse();
                // Now we have a new path, let's check if we meet the target?
                let result = checksol(target, data, nonce_bytes, &mut path_vec.clone());
                if !result.is_empty(){                                              
                    return (nonce_bytes, result, similar_path, true)
                }
                cntn = cntn + 1;
                if cntn > batchsize { // reverse times up!
                        break 'outdoor;
                }
            } else {
                continue;
            }
        }
    }
   
    (
        nonce_bytes,
        pairs,
        similar_path,
        true,
    )
}



// Check if the sol is valid.
pub fn checksol(
    target: &[u8],
    data: &[u8],
    /*hash_first: &[u8],*/
    nonce_bytes: [u8; 4],
    path: &mut Vec<u16>,
) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut full_path = path.clone();
    if full_path.len() < GRAPH_SIZE.into() {
        full_path.resize(GRAPH_SIZE.into(), u16::MAX);
    }
    let vdf_solution_solved: Vec<u8> = full_path
        .iter()
        .flat_map(|&val| val.to_le_bytes())
        .collect();

    let data_with_vdf_solved = [data, &nonce_bytes, &vdf_solution_solved].concat();
    let hash2 = Sha256::digest(&data_with_vdf_solved);
    let final_hash_reversed = hash2.iter().rev().cloned().collect::<Vec<u8>>();
    if meets_target(&final_hash_reversed, &target) {
        println!(
            "{}",
            format!("hash = {}", hex::encode(final_hash_reversed))
                .bold()
                .purple()
        );
        pairs.push((hex::encode(nonce_bytes), hex::encode(vdf_solution_solved)));
    }
    pairs
}


