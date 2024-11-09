use primitive_types::U256;
use rand_mt::Mt19937GenRand64;
use std::{iter, time::{Duration, Instant}};

pub const GRAPH_SIZE: u16 = 2008;
pub const PACKED_GRAPH_SIZE : usize = (((GRAPH_SIZE as usize) * (GRAPH_SIZE as usize) / 2) + 31) / 32;
pub const MAX_CHECKS : usize = 7000;

struct GraphData {
    pub d : [u32; PACKED_GRAPH_SIZE],
    pub _size : u32
}

impl GraphData {
    pub fn new(size : u32) -> Self {
        GraphData {
            d : [0; PACKED_GRAPH_SIZE],
            _size : size
        }
    }

    pub fn coord_to_idx(&self, i : u32, j : u32) -> u32 {
        if i <= j {
            return i * self._size - ((i*(i + 1)) >> 1) + (j-(i+1)); 
        }

        return j * self._size - ((j*(j + 1)) >> 1) + (i - (j+1));            
    }

    pub fn get(&self, i : u32, j : u32) -> bool {
        let idx : u32 = self.coord_to_idx(i,j);
        let bit_pos : u32 = idx & 0x1f;

        let val : u32 = self.d[(idx >> 5) as usize] & (1 << (31-bit_pos));
        return val != 0; 
    }    
}

pub struct HCGraphUtil {
    start_time: Instant,
    vdf_bailout: u64,
    nchecks : usize
}

impl HCGraphUtil {
    pub fn new(vdf_bailout: Option<u64>) -> Self {
        let bailout_timer: u64 = match vdf_bailout {
            Some(timer) => timer,
            None => 1000, // default to 1 second
        };
        HCGraphUtil {
            start_time: Instant::now(),
            vdf_bailout: bailout_timer,
            nchecks: 0
        }
    }

    fn hex_to_u64(&self, hex_string: &str) -> u64 {
        u64::from_str_radix(hex_string, 16).expect("Failed to convert hex to u64")
    }

    fn read_le_u64(&self, bytes: &[u8]) -> u64 {
        let arr: [u8; 8] = bytes[..8].try_into().expect("Slice with incorrect length");
        u64::from_le_bytes(arr)
    }

    fn get_u64(&self, data: &[u8], pos: usize) -> u64 {
        self.read_le_u64(&data[pos * 8..(pos + 1) * 8])
    }

    fn extract_seed_from_hash(&self, hash: &U256) -> u64 {
        let bytes = hash.to_little_endian();
        self.get_u64(&bytes, 0)
    }

    fn get_grid_size_v2(&self, hash: &U256) -> u16 {
        let hash_hex = format!("{:064x}", hash);
        let grid_size_segment = &hash_hex[0..8];
        let grid_size: u64 = self.hex_to_u64(grid_size_segment);

        let min_grid_size = 2000u64;
        let max_grid_size = GRAPH_SIZE as u64;

        let mut grid_size_final = min_grid_size + (grid_size % (max_grid_size - min_grid_size));
        if grid_size_final > max_grid_size {
            grid_size_final = max_grid_size;
        }
        grid_size_final as u16
    }

    fn generate_graph_v2(&self, hash: &U256, grid_size: u16) -> Vec<Vec<bool>> {
        let grid_size = grid_size as usize;
        let mut graph = vec![vec![false; grid_size]; grid_size];
        let num_edges = (grid_size * (grid_size - 1)) / 2;
        let bits_needed = num_edges;

        let seed = self.extract_seed_from_hash(hash);
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

    fn my_generate_graph_v2(&self, hash: &U256, grid_size: u16) -> GraphData
    {
        let mut graph = GraphData::new(grid_size as u32);
        let seed = self.extract_seed_from_hash(hash);
        let mut prng = Mt19937GenRand64::from(seed.to_le_bytes());
        for dw in 0..PACKED_GRAPH_SIZE {
            graph.d[dw] = (prng.next_u64() & 0xFFFFFFFF) as u32;
        }
        return graph;
    }

    fn my_hamiltonian_cycle_util(&mut self, graph : &GraphData,
        path : &mut [u16],
        pos : usize, 
        freev : &mut [u16]) -> bool
    {
        self.nchecks = self.nchecks + 1;
        if self.nchecks > MAX_CHECKS {
            return false;
        }

        if pos == graph._size as usize {
            if graph.get(path[pos - 1] as u32, path[0] as u32) {
                return true;
            } else {
                return false;
            }
        }

        let mut prev_pos : u16 = 0;

        loop {
            let v : u16 = freev[prev_pos as usize];
            if v as u32 == graph._size {
                break;        
            }

            if graph.get(path[pos - 1] as u32, v as u32) {
                path[pos] = v;
                freev[prev_pos as usize] = freev[v as usize];

                if self.my_hamiltonian_cycle_util(&graph, path, pos + 1, freev) {
                    return true;
                }
                if self.nchecks > MAX_CHECKS {
                    return false;
                }                

                freev[prev_pos as usize] = v;
                
            }

            prev_pos = v;
        }

        return false;
    }


    pub fn my_find_hamiltonian_cycle_v2(&mut self, graph_hash: U256) -> Vec<u16> {
        let grid_size = self.get_grid_size_v2(&graph_hash);


        let graph = self.my_generate_graph_v2(&graph_hash, grid_size);

        let mut path = vec![u16::MAX; grid_size as usize];
        path[0] = 0;

        self.start_time = Instant::now();
        self.nchecks = 0;

        let mut freev = vec![u16::MAX; graph._size as usize];
        for i in 0..freev.len() {
            freev[i] = (i+1) as u16;
        }

        if !self.my_hamiltonian_cycle_util(&graph, &mut path, 1, &mut freev) {
            return vec![];
        }
        path
    }


    fn _opt(&self, hash: &U256, grid_size: u16) -> Vec<Vec<bool>> {
        let grid_size = grid_size as usize;
        let mut graph = vec![vec![false; grid_size]; grid_size];
        let num_edges = (grid_size * (grid_size - 1)) / 2;

        let seed = self.extract_seed_from_hash(hash);
        let mut prng = Mt19937GenRand64::from(seed.to_le_bytes());

        // 位流生成作为一次性流，而不是存储在一个容器中
        let mut bit_iterator = iter::from_fn(|| {
            let random_bits_32: u32 = (prng.next_u64() & 0xFFFFFFFF) as u32;
            Some(random_bits_32)
        })
        .flat_map(|bits| (0..32).rev().map(move |j| ((bits >> j) & 1) == 1))
        .take(num_edges);

        // 遍历整个图只为抓取必要的位
        for i in 0..grid_size {
            for j in (i + 1)..grid_size {
                if let Some(edge_exists) = bit_iterator.next() {
                    graph[i][j] = edge_exists;
                    graph[j][i] = edge_exists;
                }
            }
        }

        graph
    }

    fn is_safe(&self, v: u16, graph: &Vec<Vec<bool>>, path: &[u16], pos: usize) -> bool {
        if !graph[path[pos - 1] as usize][v as usize] {
            return false;
        }

        for i in 0..pos {
            if path[i] == v {
                return false;
            }
        }

        true
    }

    fn is_safe_vp(&self, v: u16, graph: &Vec<Vec<bool>>, path: &[u16], pos: usize) -> bool {
        if pos == 0 || !graph[path[pos - 1] as usize][v as usize] {
            return false;
        }

        for &node in &path[..pos] {
            if node == v {
                return false; // 如果已经访问过，返回 false
            }
        }

        true
    }

    fn hamiltonian_cycle_util(
        &mut self,
        graph: &Vec<Vec<bool>>,
        path: &mut [u16],
        pos: usize,
    ) -> bool {
        let elapsed = self.start_time.elapsed();
        if elapsed > Duration::from_millis(self.vdf_bailout) {
            return false;
        }

        if pos == graph.len() {
            return graph[path[pos - 1] as usize][path[0] as usize];
        }

        for v in 1..graph.len() as u16 {
            if self.is_safe(v, graph, path, pos) {
                path[pos] = v;

                if self.hamiltonian_cycle_util(graph, path, pos + 1) {
                    return true;
                }

                path[pos] = u16::MAX;
            }
        }

        false
    }

    fn hamiltonian_cycle_util_vp(
        &mut self,
        graph: &Vec<Vec<bool>>,
        path: &mut [u16],
        visited: &mut Vec<bool>,
    ) -> bool {
        let mut position_vertex_stack: Vec<(usize, usize)> = Vec::new();
        let mut pos = 1;
        let mut vertex = 1;
        
        loop {
            let elapsed = self.start_time.elapsed();
            if elapsed > Duration::from_millis(self.vdf_bailout) {
                return false;
            }
    
            // Check if the cycle completed
            if pos == graph.len() {
                if graph[path[pos - 1] as usize][path[0] as usize] {
                    return true;
                }
                // If not a valid cycle, backtrack
                if let Some((prev_pos, prev_vertex)) = position_vertex_stack.pop() {
                    visited[path[prev_pos] as usize] = false;
                    path[prev_pos] = u16::MAX;
                    pos = prev_pos;
                    vertex = prev_vertex + 1;
                    continue;
                }
                return false;
            }
    
            // Try to find next valid vertex
            while vertex < graph.len() {
                if !visited[vertex] && self.is_safe_vp(vertex as u16, graph, path, pos) {
                    path[pos] = vertex as u16;
                    visited[vertex] = true;
                    position_vertex_stack.push((pos, vertex));
                    pos += 1;
                    vertex = 1;
                    break;
                }
                vertex += 1;
            }
    
            // If no valid vertex found, backtrack
            if vertex >= graph.len() {
                if let Some((prev_pos, prev_vertex)) = position_vertex_stack.pop() {
                    visited[path[prev_pos] as usize] = false;
                    path[prev_pos] = u16::MAX;
                    pos = prev_pos;
                    vertex = prev_vertex + 1;
                } else {
                    return false;
                }
            }
        }
    }

    pub fn find_hamiltonian_cycle_v2(&mut self, graph_hash: U256) -> Vec<u16> {
        let grid_size = self.get_grid_size_v2(&graph_hash);
        let graph = self.generate_graph_v2(&graph_hash, grid_size);

        let mut path = vec![u16::MAX; graph.len()];
        path[0] = 0;
        self.start_time = Instant::now();

        if !self.hamiltonian_cycle_util(&graph, &mut path, 1) {
            return vec![];
        }
        path
    }

    pub fn find_hamiltonian_cycle_vp(&mut self, graph_hash: U256) -> Vec<u16> {
        let grid_size = self.get_grid_size_v2(&graph_hash);
        let graph = self._opt(&graph_hash, grid_size);

        let mut path = vec![u16::MAX; graph.len()];
        path[0] = 0;
        let mut visited = vec![false; graph.len()];
        visited[0] = true;
        self.start_time = Instant::now();

        if !self.hamiltonian_cycle_util_vp(&graph, &mut path, &mut visited) {
            return vec![];
        }
        path
    }
}
