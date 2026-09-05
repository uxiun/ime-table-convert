use std::{fs::{File, read_to_string}, io::{self, BufReader}, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value::{self, Array};


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct ChDictWord {
  word: &'static str,
  pinyin: &'static str,
  abbr: &'static str,
  explanation: &'static str,
}

// impl ChDictWord {
//   fn from_word_json<P: AsRef<Path>>(path: P) -> io::Result<Vec<ChDictWord>> {
//     // let file = File::open(path)?;
//     // let mut reader = BufReader::new(file);

//     let s = read_to_string(path)?;
//     let value: Value = serde_json::from_str(&s)?;
//     if let Array(words) = value {
//       words.into_iter()
//         .map(|w| )
//     }
//   }
// }
