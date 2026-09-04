mod dict;
use std::{collections::HashMap, hash::Hash};

pub fn hashmap_reverse<K: Copy,V: Eq + Hash + Copy>(hashmap: &HashMap<K,V>) -> HashMap<V,K> {
  hashmap.into_iter()
    .map(|(k,v)| (*v, *k))
    .collect()
}

