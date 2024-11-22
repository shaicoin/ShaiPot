use std::collections::HashSet;

use primitive_types::U256;
use rand_mt::Mt19937GenRand64;


pub const GRAPH_SIZE: u16 = 2008;



fn read_le_u64(bytes: &[u8]) -> u64 {
    let arr: [u8; 8] = bytes[..8].try_into().expect("Slice with incorrect length");
    u64::from_le_bytes(arr)
}

fn get_u64(data: &[u8], pos: usize) -> u64 {
    read_le_u64(&data[pos * 8..(pos + 1) * 8])
}

fn extract_seed_from_hash(hash: &U256) -> u64 {
    let bytes = hash.to_little_endian();
    get_u64(&bytes, 0)
}

fn hex_to_u64(hex_string: &str) -> u64 {
    u64::from_str_radix(hex_string, 16).expect("Failed to convert hex to u64")
}


fn generate_graph_v2(hash: &U256, grid_size: u16) -> Vec<Vec<bool>> {
    let grid_size = grid_size as usize;
    let mut graph = vec![vec![false; grid_size]; grid_size];
    let num_edges = (grid_size * (grid_size - 1)) / 2;
    let bits_needed = num_edges;

    let seed = extract_seed_from_hash(hash);
    let mut prng = Mt19937GenRand64::from(seed.to_le_bytes());

    let mut bit_stream = Vec::with_capacity(bits_needed);

    while bit_stream.len() < bits_needed {
        let random_bits_32: u32 = (prng.next_u64() & 0xFFFFFFFF) as u32;
        for j in (0..32).rev() {
            if bit_stream.len() >= bits_needed {
                break;
            }
            let bit = ((random_bits_32 >> j) & 1) == 1;
            bit_stream.push(bit);
        }
    }

    let mut bit_index = 0;
    for i in 0..grid_size {
        for j in (i + 1)..grid_size {
            let edge_exists = bit_stream[bit_index];
            bit_index += 1;
            graph[i][j] = edge_exists;
            graph[j][i] = edge_exists;
        }
    }

    graph
}


fn get_grid_size_v2(hash: &U256) -> u16 {
    let hash_hex = format!("{:064x}", hash);
    let grid_size_segment = &hash_hex[0..8];
    let grid_size: u64 = hex_to_u64(grid_size_segment);

    let min_grid_size = 2000u64;
    let max_grid_size = GRAPH_SIZE as u64;

    let mut grid_size_final = min_grid_size + (grid_size % (max_grid_size - min_grid_size));
    if grid_size_final > max_grid_size {
        grid_size_final = max_grid_size;
    }
    grid_size_final as u16
}


fn verify_hamiltonian_cycle(graph: &[Vec<bool>], path: &[u16]) -> bool {
    let mut path_size = 0;
    if let Some(pos) = path.iter().position(|&x| x == u16::MAX) {
        path_size = pos;
    } else {
        path_size = path.len();
    }

    let n = graph.len();

    // Check if path contains all vertices exactly once
    if path_size != n {
        return false;
    }

    let vertices_in_path: HashSet<_> = path.iter().take(path_size).cloned().collect();
    if vertices_in_path.len() != n {
        return false;
    }

    // Check if the path forms a cycle
    for i in 1..n {
        if !graph[path[i - 1] as usize][path[i] as usize] {
            return false;
        }
    }

    // Check if there's an edge from the last to the first vertex to form a cycle
    if !graph[path[n - 1] as usize][path[0] as usize] {
        return false;
    }

    true
}


pub fn verify(hash : U256, path_vec: &[u16]) -> bool{
    let grid_size = get_grid_size_v2(&hash);
    let graph = generate_graph_v2(&hash, grid_size);
    verify_hamiltonian_cycle(&graph, path_vec)
}