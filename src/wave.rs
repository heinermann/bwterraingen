use crate::{bitset::BitSet, rules::Rules};
use crate::{DIRECTIONS, N};
use anyhow::anyhow;
use anyhow::Result;
use cached::proc_macro::cached;
use instant::Instant;
use rand::distributions::Uniform;
use rand::prelude::ThreadRng;
use rand::prelude::{Distribution, SliceRandom};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Wave {
    pub width: isize,
    pub height: isize,
    array: Vec<BitSet<N>>,
    rules: Rc<[HashMap<usize, BitSet<N>>; 4]>,
    inverse_mapping: Rc<HashMap<usize, u16>>,
}

#[cached(
    key = "(BitSet<N>, usize)",
    convert = r#"{ (bitset, ordinal) }"#,
    size = 100000
)]
fn get_allowed_rules(
    bitset: BitSet<N>,
    ordinal: usize,
    rules: &[HashMap<usize, BitSet<N>>; 4],
) -> BitSet<N> {
    // trace!("get_allowed_rules. ordinal: {ordinal}");
    let mut allowed = BitSet::new();

    for x in bitset.iter().filter_map(|x| rules[ordinal].get(&x)) {
        allowed.union(x);
    }
    allowed
}

impl Wave {
    pub fn new(width: usize, height: usize, rules: &Rules) -> Wave {
        // remap rules so they are not sparse
        let mut all_tiles = BitSet::<N>::new();
        let mut mapping = HashMap::new();
        let mut inverse_mapping = HashMap::new();

        let mut new_rules = [
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        ];

        inverse_mapping.insert(mapping.len(), 0);
        mapping.insert(0, mapping.len());

        // build up mapping tables, this should contain all keys.
        for (ordinal, _) in DIRECTIONS.iter().enumerate() {
            for &key in rules.ruleset[ordinal].keys() {
                if !mapping.contains_key(&key) {
                    inverse_mapping.insert(mapping.len(), key);
                    mapping.insert(key, mapping.len());
                }
            }
        }

        // convert it to dense format
        for (ordinal, _) in DIRECTIONS.iter().enumerate() {
            let old_rule = &rules.ruleset[ordinal];
            let new_rule = &mut new_rules[ordinal];

            let mut rule = HashMap::new(); // HashMap<usize, BitSet<N>>
            for (tile, allowed_tiles) in old_rule {
                let mut allowed_tiles_remapped = BitSet::<N>::new();
                for allowed_tile in allowed_tiles {
                    all_tiles.set(mapping[allowed_tile]);
                    allowed_tiles_remapped.set(mapping[allowed_tile]);
                }
                rule.insert(mapping[tile], allowed_tiles_remapped);
            }

            *new_rule = rule;
        }

        //let wave =

        // wave.constrain();

        Wave {
            width: width as isize,
            height: height as isize,
            array: vec![all_tiles; width * height],
            rules: Rc::new(new_rules),
            inverse_mapping: Rc::new(inverse_mapping),
        }
    }

    pub fn print_wave(&self) {
        info!("wave:");
        for y in 0..self.height {
            let mut output = String::new();
            for x in 0..self.width {
                output = format!(
                    "{}{:4x}",
                    output,
                    self.array[(x + y * self.width) as usize].pop_cnt()
                );
            }
            info!("        {}", output);
        }
    }

    pub fn render(&self) -> Vec<u16> {
        let mut ret = Vec::new();
        for v in &self.array {
            if v.pop_cnt() > 1 || v.pop_cnt() == 0 {
                ret.push(self.inverse_mapping[&0]);
            } else {
                ret.push(self.inverse_mapping[&v.iter().next().unwrap()]);
            }
        }
        ret
    }

    pub fn is_done(&self) -> bool {
        for v in &self.array {
            if v.pop_cnt() != 1 {
                return false;
            }
        }

        true
    }

    // pub fn constrain(&mut self) -> bool {
    //     for y in 0..self.height {
    //         for x in 0..self.width {
    //             if !self.propagate((x + y * self.width) as usize) {
    //                 return false;
    //             }
    //         }
    //     }

    //     true
    // }

    pub fn propagate(&mut self, start: usize, _rng: &mut ThreadRng) -> bool {
        #[derive(Copy, Clone, Eq, PartialEq, Debug)]
        struct Node {
            target_index: usize,
            pop_cnt: usize,
        }

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                // Notice that the we flip the ordering on costs.
                // In case of a tie we compare positions - this step is necessary
                // to make implementations of `PartialEq` and `Ord` consistent.
                other.pop_cnt.cmp(&self.pop_cnt)
            }
        }

        // `PartialOrd` needs to be implemented as well.
        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        //let mut btree = std::collections::BTreeSet::<Node>::new();
        // let mut heap = std::collections::BinaryHeap::<Node>::new();
        // //let mut stack = VecDeque::new();
        // heap.push(Node {
        //     target_index: start,
        //     pop_cnt: self.array[start].pop_cnt(),
        // });
        let mut vec = Vec::<Node>::new();

        // btree.insert(Node {
        //     target_index: start,
        //     pop_cnt: self.array[start].pop_cnt(),
        // });

        vec.push(Node {
            target_index: start,
            pop_cnt: self.array[start].pop_cnt(),
        });

        while vec.len() > 0 {
            //info!("len: {}, heap: {:?}", vec.len(), vec);
            let chosen = vec.pop().unwrap();
            //info!("chosen: {:?}", chosen);
            let chosen_index = chosen.target_index;

            for (ordinal, direction) in DIRECTIONS.iter().enumerate() {
                let (target_x, target_y) = (
                    chosen_index as isize % self.width + direction.0,
                    chosen_index as isize / self.width + direction.1,
                );

                // check if neighbor is outside the bounds.
                if target_x < 0 || target_x >= self.width || target_y < 0 || target_y >= self.height
                {
                    continue;
                }

                let target = (target_x + target_y * self.width) as usize;

                // let rules = &self.rules[ordinal];

                // let mut allowed = BitSet::new();
                // for x in self.array[chosen_index]
                //     .iter()
                //     .filter_map(|x| self.rules[ordinal].get(&x))
                // {
                //     allowed.union(x);
                // }
                let allowed = get_allowed_rules(self.array[chosen_index], ordinal, &self.rules);

                let old_len = self.array[target].pop_cnt();
                self.array[target].intersect(&allowed);
                let new_len = self.array[target].pop_cnt();

                if new_len == 0 {
                    // info!("ordinal: {ordinal}, chosen_index: {chosen_index}");
                    return false;
                }

                if new_len < old_len {
                    // stack.push_back(target);
                    // heap.retain(|x| x.target_index != target);
                    // heap.heap.push(Node {
                    //     target_index: target,
                    //     pop_cnt: self.array[target].pop_cnt(),
                    // });

                    let mut was_found = false;
                    for i in &mut vec {
                        if i.target_index == target {
                            i.pop_cnt = new_len;
                            was_found = true;
                            break;
                        }
                    }
                    if !was_found {
                        vec.push(Node {
                            target_index: target,
                            pop_cnt: new_len,
                        });
                    }

                    vec.sort_by(|a, b| b.pop_cnt.cmp(&a.pop_cnt));

                    //btree.get(t
                }
            }
        }

        true
    }

    pub fn get_entropy_indices_in_order(
        &mut self,
        mut rng: &mut ThreadRng,
    ) -> Result<Vec<(usize, usize)>> {
        // TODO: use real entropy instead of number of possibilities. Requires tile weights.
        let entropies = self
            .array
            .iter()
            .map(|x| x.pop_cnt())
            .enumerate()
            .filter(|&x| x.1 > 1)
            .collect::<Vec<(usize, usize)>>();

        anyhow::ensure!(entropies.len() >= 1);

        let minimum_entropy = entropies
            .iter()
            .map(|x| x.1)
            .min()
            .ok_or(anyhow!("something"))?;
        let mut entropies: Vec<_> = entropies
            .into_iter()
            .filter(|v| v.1 <= minimum_entropy)
            .collect();

        entropies.shuffle(&mut rng);

        entropies.sort_unstable_by_key(|x| x.1);

        anyhow::Ok(entropies)
    }

    pub fn collapse_index(&mut self, mut rng: &mut ThreadRng, index: usize) {
        let chosen_bit = Uniform::from(0..self.array[index].pop_cnt()).sample(&mut rng);
        self.array[index].clear_all_except_nth_set_bit(chosen_bit);
    }

    pub fn logical_conclusion<F: Fn(&Wave)>(&self, update: &F) -> Result<Wave> {
        let mut rng = rand::thread_rng();
        let mut last_time = Instant::now();

        anyhow::Ok(loop {
            let mut fail_count = 0;

            info!("RESTART");

            if let Ok(wave) = process_wave(
                self.clone(),
                &mut rng,
                0,
                &mut last_time,
                &mut fail_count,
                update,
            ) {
                break wave;
            } else {
                continue;
            };
        })
    }
}

pub fn process_wave<F: Fn(&Wave)>(
    mut wave: Wave,
    mut rng: &mut ThreadRng,
    depth: usize,
    last_time: &mut Instant,
    fail_count: &mut usize,
    update: &F,
) -> Result<Wave> {
    if Instant::now().duration_since(*last_time).as_millis() > 100 {
        *last_time = Instant::now();
        info!("depth: {depth:5}, fail_count: {fail_count:6}",);
        update(&wave);
    }

    if wave.is_done() {
        return anyhow::Ok(wave);
    }

    let indices = wave.get_entropy_indices_in_order(&mut rng)?;

    for (index, _pop_cnt) in &indices {
        if Instant::now().duration_since(*last_time).as_millis() > 100 {
            *last_time = Instant::now();
            info!("depth: {depth:5}, fail_count: {fail_count:6}",);
            update(&wave);
        }

        if *fail_count > 5000 {
            return anyhow::Ok(wave);
        }

        let mut wave = wave.clone();

        wave.collapse_index(&mut rng, *index);

        if !wave.propagate(*index, rng) {
            continue;
        }

        if let Ok(ret) = process_wave(wave.clone(), rng, depth + 1, last_time, fail_count, update) {
            return anyhow::Ok(ret);
        } else {
            *fail_count += 1;
        }
    }

    anyhow::Result::Err(anyhow!("Couldn't quite get there"))
}
