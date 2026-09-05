use std::{
  cmp::Ordering,
  collections::HashMap,
  fs::{self, OpenOptions, read_to_string},
  hash::Hash,
  io::{self, BufWriter, Write},
  path::Path,
};

use ime_table_convert::hashmap_reverse;
use pinyin::{ToPinyin, ToPinyinMulti};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CQKM_FORM_LAYOUT: &str = "
  wa em rp tn     yb uy ih og
  sd dk fi gc     hv ju kj lf
  zq xr cw .e  bt ,x mo nl vs
";

const CQKM_INITIAL_LAYOUT: &str = "
  wp eb rf tm    yy ux ij oq
  st dd fl gn    hr ju ki lo
  zk xg ch .a be ,w ms nz vc
";

const CJ5_LAYOUT: &str =
  "az bq cg dh et fn gu hv ii jb km lr ms ne of pw qx rl so tp ud vc wy xk ya zj";
//                           v                    v
// "az bq cg dh et fn gu hv ii jx km lr ms ne of pw qb rl so tp ud vc wy xk ya zj";
// 参照元がそもそもb/qの綴が入れ替わっちゃってる状態だったっぽい

fn to_char_dict(char_pairs_str: &str) -> HashMap<char, char> {
  char_pairs_str
    .split_whitespace()
    .filter_map(|word| {
      let mut cs = word.chars();
      match (cs.next(), cs.next()) {
        (Some(i), Some(k)) => Some((i, k)),
        _ => None,
      }
    })
    .collect()
}

fn main() {
  zhwords_json("json/cqkm-word-array.json");
}

fn zhwords_json<P: AsRef<Path>>(json_path: P) -> io::Result<()> {
  let words = getZh("json/cqkm-word.json")?;
  let s = serde_json::to_string(&words)?;
  fs::write(json_path, s)
}

fn main_json() -> io::Result<()> {
  let res = get_hanzis(
    &["json/cqkm-char.json", "json/cqkm-char-extra.json"],
    &[("Cangjie5/Cangjie5_special.txt", false)],
  );

  if let Err(e) = &res {
    println!("{}", e)
  }
  let mut hans = res.unwrap();
  dbg!(hans.len());
  let cqkm_count = hans.iter().filter(|z| z.cqkm_form.is_some()).count();
  dbg!(cqkm_count);

  let initial_layout = to_char_dict(CQKM_INITIAL_LAYOUT);
  let form_layout = to_char_dict(CQKM_FORM_LAYOUT);
  let cj5_layout = to_char_dict(CJ5_LAYOUT);

  hans
    .iter_mut()
    .for_each(|z| z.fill_cqkm_initials(&initial_layout));
  hans
    .iter_mut()
    .for_each(|h| h.apply_custom_layout(&cj5_layout, &initial_layout, &form_layout));

  // for key in "qwertyuiopasdfghjkl;zxc.b,mnv".split("") {
  //   let found = hans.iter().find(|z| z.cj5.iter().any(|s| s == key));
  //   if let Some(z) = found {
  //     println!("{}: {}", key, z.zh);
  //   }
  // }

  // run_stat(&hans);

  // let words_json: String = read_to_string("json/cqkm-word.json")?;
  // let words: Vec<CqkmWord> = serde_json::from_str(&words_json)?;
  // let mut words_counter: HashMap<char, u64> = HashMap::new();
  // count_map(&mut words_counter, &words, |w| w.zh.chars());

  // println!("\n\n-----run_stat_weighted-------");
  // run_stat_weighted(&hans, &words_counter);

  // return Ok(());

  // let mut codes: Vec<Code> = hans.iter().flat_map(|z| z.codes()).collect();
  let mut codes: Vec<Code> = hans
    .iter()
    .flat_map(|z| {
      let mut codes = z.codes();
      codes.append(&mut z.cqkm_xy_codes());
      codes
    })
    .collect();
  // let mut codes: Vec<Code> = vec![];

  let s = read_to_string("json/cqkm-char-shortcuts.json")?;
  let shortcuts: Vec<CqkmWord> = serde_json::from_str(&s)?;
  dbg!(shortcuts.len());

  let xhs = xhs_map(&initial_layout);
  let hanm: HashMap<&String, &Hanzi> = hans.iter().map(|z| (&z.zh, z)).collect();
  let initial_mapper = hashmap_reverse(&initial_layout);
  let form_mapper = hashmap_reverse(&form_layout);

  let initial_mapped = |original: &str| {
    original
      .chars()
      .filter_map(|c| initial_mapper.get(&c))
      .collect::<String>()
  };

  let form_mapped = |original: &str| {
    original
      .chars()
      .filter_map(|c| form_mapper.get(&c))
      .collect::<String>()
  };

  for short in shortcuts {
    let initial = &short.spell[0..1];
    let code = if "zcs".contains(initial) {
      let xh = format!("{initial}h");
      if hanm
        .get(&short.zh)
        .map(|z| z.pinyins.iter().any(|p| p.starts_with(&xh)))
        .unwrap_or(false)
      {
        format!(
          "{}{}",
          xhs.get(xh.as_str()).unwrap(),
          form_mapped(&short.spell[1..])
        )
      } else {
        format!(
          "{}{}",
          initial_mapped(&short.spell[0..1]),
          form_mapped(&short.spell[1..])
        )
      }
    } else {
      format!(
        "{}{}",
        initial_mapped(&short.spell[0..1]),
        form_mapped(&short.spell[1..])
      )
    };

    codes.push(Code {
      zh: short.zh,
      code,
      schema: "cqkm".to_string(),
      nth: 0,
    });
  }

  codes.sort();

  json_array_write("json/Cangjie5_special_hans_custom.json", &hans)?;
  json_array_write("json/Cangjie5_special_codes_cqkmxy.json", &codes)?;

  Ok(())
}

fn count_map<
  U,
  T: IntoIterator<Item = U>,
  C: Eq + Hash,
  X: IntoIterator<Item = C>,
  F: Fn(U) -> X,
>(
  counter: &mut HashMap<C, u64>,
  it: T,
  get: F,
) {
  for item in it.into_iter() {
    let cs = get(item);
    for c in cs.into_iter() {
      counter.entry(c).and_modify(|i| *i += 1).or_insert(1);
    }
  }
}

fn count_weighted<
  C: Eq + Hash,
  D: Eq + Hash,
  T,
  S: IntoIterator<Item = T>,
  F: Fn(&T) -> C,
  I: IntoIterator<Item = D>,
  G: Fn(T) -> I,
>(
  weights: &HashMap<C, u64>,
  counter: &mut HashMap<D, u64>,
  src: S,
  get_weight_key: F,
  get_count_targets: G,
) {
  for item in src.into_iter() {
    let w = *weights.get(&get_weight_key(&item)).unwrap_or(&1);
    let ds = get_count_targets(item);
    for d in ds.into_iter() {
      counter.entry(d).and_modify(|i| *i += w).or_insert(w);
    }
  }
}

fn run_stat(hans: &[Hanzi]) {
  let mut hans_cqkm_form: HashMap<char, u64> = HashMap::new();
  let mut hans_cqkm_initial: HashMap<char, u64> = HashMap::new();
  let mut hans_cj5: HashMap<char, u64> = HashMap::new();

  for h in hans {
    if let Some(form) = &h.cqkm_form {
      char_count(&mut hans_cqkm_form, form.as_str());
    }
    char_count(&mut hans_cj5, h.cj5.join("").as_str());
    char_count(&mut hans_cqkm_initial, h.cqkm_initials.join("").as_str());
  }

  println!("\nhans_cj5");
  char_count_print(&hans_cj5);
  println!("\nhans_cqkm_form");
  char_count_print(&hans_cqkm_form);
  println!("\nhans_cqkm_initial");
  char_count_print(&hans_cqkm_initial);
}
fn run_stat_weighted(hans: &[Hanzi], weights: &HashMap<char, u64>) {
  let mut hans_cqkm_form: HashMap<char, u64> = HashMap::new();
  let mut hans_cqkm_initial: HashMap<char, u64> = HashMap::new();
  let mut hans_cj5: HashMap<char, u64> = HashMap::new();

  count_weighted(
    weights,
    &mut hans_cj5,
    hans,
    |h| h.zh.chars().next().unwrap(),
    |h| h.cj5.join("").chars().collect::<Vec<_>>(),
  );

  count_weighted(
    weights,
    &mut hans_cqkm_form,
    hans,
    |h| h.zh.chars().next().unwrap(),
    |h| {
      h.cqkm_form
        .clone()
        .unwrap_or_default()
        .chars()
        .take(2)
        .collect::<Vec<_>>()
    },
  );

  count_weighted(
    weights,
    &mut hans_cqkm_initial,
    hans,
    |h| h.zh.chars().next().unwrap(),
    |h| h.cqkm_initials.join("").chars().take(1).collect::<Vec<_>>(),
  );

  // for h in hans {
  //   if let Some(form) = &h.cqkm_form {
  //     char_count(&mut hans_cqkm_form, form.as_str());
  //   }
  //   char_count(&mut hans_cj5, h.cj5.join("").as_str());
  //   char_count(&mut hans_cqkm_initial, h.cqkm_initials.join("").as_str());
  // }

  println!("\nhans_cj5");
  char_count_print(&hans_cj5);
  println!("\nhans_cqkm_form");
  char_count_print(&hans_cqkm_form);
  println!("\nhans_cqkm_initial");
  char_count_print(&hans_cqkm_initial);
}

fn char_count_print(counter: &HashMap<char, u64>) {
  let mut v = counter.iter().collect::<Vec<_>>();
  let count_sum: u64 = v.iter().map(|(_, n)| **n).sum();

  v.sort_by(|i, j| i.1.partial_cmp(j.1).expect("partial_cmp"));
  for (c, x) in v {
    let percent: f64 = *x as f64 * 100_f64 / count_sum as f64;
    println!("{c}: ({:.2}%) {}", percent, x);
  }
}

fn char_count(counter: &mut HashMap<char, u64>, s: &str) {
  for c in s.chars() {
    counter.entry(c).and_modify(|i| *i += 1).or_insert(1);
  }
}

const XHS: [(&str, &str); 3] = [("zh", "i"), ("ch", "o"), ("sh", "u")];
fn xhs_map(initial_layout: &HashMap<char, char>) -> HashMap<&str, char> {
  let initial_mapper = hashmap_reverse(initial_layout);
  XHS
    .map(|(k, v)| (k, *initial_mapper.get(&v.chars().next().unwrap()).unwrap()))
    .into_iter()
    .collect()
}

fn json_array_write<T: Serialize>(path: &str, items: &[T]) -> io::Result<()> {
  let f = OpenOptions::new()
    .create(true)
    .truncate(true)
    .write(true)
    .open(path)?;

  let mut b = BufWriter::new(f);
  b.write_all(b"[")?;
  for (count, item) in items.iter().enumerate() {
    let s = serde_json::to_string(item)?;
    let line = format!("{}{}\n", if count > 0 { "," } else { "" }, s);
    b.write_all(line.as_bytes())?;
  }
  b.write_all(b"]\n")?;
  b.flush()?;

  Ok(())
}

fn pinyin_ascii(tone_num_end_pinyin: &str) -> String {
  tone_num_end_pinyin.replace('ü', "v").replace('ê', "e")
}

#[derive(Debug, Serialize, Deserialize)]
struct CqkmChar {
  zh: String,
  code: String,
  initials: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CqkmWord {
  zh: String,
  spell: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Code {
  zh: String,
  code: String,
  schema: String,
  nth: i32,
}

impl Code {
  // fn set_nth(codes: &mut [Self]) {
  //   let mut last = (None, None);
  //   let mut nth = 0;
  //   for code in codes {
  //     if let (Some(c), Some(schema)) = last {
  //       if code.code == c &&
  //     }
  //   }
  // }
}

impl Ord for Code {
  fn cmp(&self, other: &Self) -> Ordering {
    [
      self.schema.cmp(&other.schema),
      self.code.cmp(&other.code),
      self.zh.cmp(&other.zh),
      self.nth.cmp(&other.nth),
    ]
    .into_iter()
    .reduce(|r, o| match r {
      Ordering::Equal => o,
      ord => ord,
    })
    .unwrap()
  }
}

impl PartialOrd for Code {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Hanzi {
  zh: String,
  pinyins: Vec<String>, // pin1yin1
  cj5: Vec<String>,
  cqkm_form: Option<String>,
  cqkm_initials: Vec<String>,
}

fn is_vowel<'a>(c: &'a char) -> bool {
  ['a', 'e', 'i', 'o', 'u'].contains(c)
}
fn is_consonant<'a>(c: &'a char) -> bool {
  "qwertyuiopasdfghjklzxcbmnv"
    .chars()
    .filter(|c| !is_vowel(c))
    .any(|x| &x == c)
}

impl Ord for Hanzi {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let cmp = |a: &str, b: &str| {
      let la = a.len();
      let lb = b.len();
      if la == lb { a.cmp(b) } else { la.cmp(&lb) }
    };

    let cja = self.cj5.iter().min_by(|a, b| cmp(a, b));

    let cjb = other.cj5.iter().min_by(|a, b| cmp(a, b));

    let ord = match (cja, cjb) {
      (Some(a), Some(b)) => cmp(a, b),
      (Some(_), None) => Ordering::Less,
      (None, Some(_)) => Ordering::Greater,
      (None, None) => Ordering::Equal,
    };

    if let Ordering::Equal = ord {
      self.zh.cmp(&other.zh)
    } else {
      ord
    }
  }
}

impl PartialOrd for Hanzi {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Hanzi {
  fn apply_custom_layout(
    &mut self,
    cj5_layout: &HashMap<char, char>,
    initial_layout: &HashMap<char, char>,
    form_layout: &HashMap<char, char>,
  ) {
    let cj5_mapper = hashmap_reverse(cj5_layout);
    let initial_mapper = hashmap_reverse(initial_layout);
    let form_mapper = hashmap_reverse(form_layout);

    self.cqkm_initials = self
      .cqkm_initials
      .iter()
      .filter_map(|i| {
        let i1 = i
          .chars()
          .next()
          .iter()
          .flat_map(|i| initial_mapper.get(i))
          .collect::<String>();
        if i1.is_empty() {
          println!("initial_layout.get({i}) returned None!");
          None
        } else {
          Some(i1)
        }
      })
      .collect();

    self.cqkm_form = self.cqkm_form.as_ref().map(|s| {
      let f = s
        .chars()
        .filter_map(|c| form_mapper.get(&c))
        .collect::<String>();
      if f.len() != s.len() {
        panic!("cqkm_form convertion is imcomplete!: {:?}", self);
      }
      f
    });

    self.cj5.iter_mut().for_each(|codes| {
      let f = codes
        .chars()
        .filter_map(|c| cj5_mapper.get(&c))
        .collect::<String>();
      if f.len() != codes.len() {
        panic!("cj5 convertion is imcomplete! for code \"{codes}\"");
      }
      *codes = f
    })
  }

  fn codes(&self) -> Vec<Code> {
    let mut codes: Vec<Code> = self
      .cj5
      .iter()
      .map(|s| Code {
        zh: self.zh.to_string(),
        code: s.to_string(),
        schema: "cj5".to_string(),
        nth: 0,
      })
      .collect();

    if let Some(form) = &self.cqkm_form {
      let mut cqkm = self
        .cqkm_initials
        .iter()
        .map(|i| Code {
          zh: self.zh.to_string(),
          code: format!("{i}{form}"),
          schema: "cqkm".to_string(),
          nth: 0,
        })
        .collect();

      codes.append(&mut cqkm);
    }

    codes
  }

  fn cqkm_xy_codes(&self) -> Vec<Code> {
    let mut codes = vec![];
    if let Some(form) = &self.cqkm_form {
      // let mut cs: Vec<char> = form.chars().collect();
      // let (left, right) = cs.split_at(2);
      let mut cqkm = self
        .cqkm_initials
        .iter()
        .filter_map(|i| {
          let code = format!("{form}{i}");
          if code.len() < 4 {
            return None;
          }
          Some(Code {
            zh: self.zh.to_string(),
            code,
            // code: format!("{}{i}{}", left.iter().collect::<String>(), right.iter().collect::<String>()),
            schema: "cqkmxy".to_string(),
            nth: 0,
          })
        })
        .collect();

      codes.append(&mut cqkm);
    }

    codes
  }

  // fn save_as_json(hanzis: &[Self], hanzi_path: &str, spells_path: &str) -> io::Result<()> {
  //   let f = OpenOptions::new()
  //     .create(true)
  //     .truncate(true)
  //     .write(true)
  //     .open(hanzi_path)?;

  //   let mut b = BufWriter::new(f);
  //   b.write_all(b"[")?;
  //   for (count, zi) in hanzis.iter().enumerate() {
  //     let item = serde_json::to_string(zi)?;
  //     let line = format!("{}{}\n", if count > 0 { "," } else { "" }, item);
  //     b.write_all(line.as_bytes())?;
  //   }
  //   b.write_all(b"]\n")?;
  //   b.flush()?;

  //   let f = OpenOptions::new()
  //     .create(true)
  //     .truncate(true)
  //     .write(true)
  //     .open(spells_path)?;

  //   let mut b = BufWriter::new(f);

  //   for (count, code) in codes.iter().enumerate() {
  //     let item = serde_json::to_string(code)?;
  //     let line = format!("{}{}\n", if count > 0 { "," } else { "" }, item);
  //     b.write_all(line.as_bytes())?;
  //   }
  //   b.write_all(b"]\n")?;
  //   b.flush()?;

  //   Ok(())
  // }

  fn fill_cqkm_initials(&mut self, initial_layout: &HashMap<char, char>) {
    // let xhs = xhs_map(initial_layout);
    let xhs: HashMap<&str, &str> = XHS.into_iter().collect();
    let initial_mapper = hashmap_reverse(&initial_layout);

    let mut is = self
      .pinyins
      .iter()
      .map(|p| {
        if xhs.keys().any(|pat| p.starts_with(pat)) {
          &p[0..2]
        } else {
          &p[0..1]
        }
      })
      // .map(|i| xhs.get(i).unwrap_or(&i).to_string())
      .collect::<Vec<_>>();

    let mut pinyin_initials = vec![];
    for i in is {
      if !pinyin_initials.contains(&i) {
        pinyin_initials.push(i)
      }
    }

    let replace_xhs = |initial: &&str| {
      xhs
        .get(initial)
        .map(|c| c.to_string())
        .unwrap_or(initial.to_string())
    };

    if self.cqkm_initials.is_empty() {
      self.cqkm_initials.append(
        &mut pinyin_initials
          .into_iter()
          .map(|s| replace_xhs(&s))
          .collect(),
      );
    } else {
      let xhs = xhs_map(&initial_layout);
      let initials = self.cqkm_initials.clone();
      self.cqkm_initials = pinyin_initials
        .iter()
        .filter_map(|initial| match *initial {
          "sh" => Some('u'),
          "zh" => Some('i'),
          "ch" => Some('o'),
          _ => initial.chars().next(),
        })
        .map(|c| c.to_string())
        .filter(|s| initials.contains(s))
        .collect();

      let mut new_initials = vec![];
      for initial in self.cqkm_initials.iter() {
        let xh = xhs.keys().find(|xh| xh.starts_with(initial));

        match xh {
          Some(xh) => {
            if pinyin_initials.contains(xh) {
              new_initials.push(replace_xhs(xh));
            } else {
              new_initials.push(initial.to_string());
            }
          }
          None => new_initials.push(initial.to_string()),
        }
      }

      self.cqkm_initials = new_initials;
    }
  }
}

fn get_hanzis(
  paths_cqkm_char: &[&'static str],
  paths_cj5_yong_is_code_left: &[(&'static str, bool)],
) -> Result<Vec<Hanzi>, Box<dyn std::error::Error>> {
  let mut chars_cqkm: Vec<CqkmChar> = vec![];
  for path in paths_cqkm_char {
    let s = fs::read_to_string(path)?;
    let mut chars: Vec<CqkmChar> = serde_json::from_str(&s)?;
    chars_cqkm.append(&mut chars);
  }
  let cqkm: HashMap<String, CqkmChar> = chars_cqkm.into_iter().map(|c| (c.zh.clone(), c)).collect();

  let mut cj5def: HashMap<String, Vec<String>> = HashMap::new();
  for (path, is_code_left) in paths_cj5_yong_is_code_left {
    let s = fs::read_to_string(path)?;
    for line in s.lines() {
      let seps: Vec<&str> = line.split_whitespace().collect();
      let code_i = if *is_code_left { 0 } else { 1 };
      let zh_i = if *is_code_left { 1 } else { 0 };
      if seps.len() == 2
        && seps[code_i].chars().all(|c| c.is_ascii_alphabetic())
        && seps[zh_i].chars().count() == 1
      // && seps[1].chars().all(|c| c.is_alphabetic())
      {
        let spell = seps[code_i].to_string();
        let zh = seps[zh_i].to_string();
        cj5def
          .entry(zh)
          .and_modify(|v| {
            if !v.contains(&spell) {
              v.push(spell.clone());
            }
          })
          .or_insert(vec![spell]);
      }
    }
  }

  println!("cj5def.len={}", cj5def.len());

  let mut hanzis: Vec<Hanzi> = cj5def
    .into_iter()
    .filter_map(|(zh, cj5)| {
      if cj5.is_empty() {
        println!("{}'s cj5 code not found.", zh)
      }

      let pinyins: Vec<String> = zh
        .as_str()
        .to_pinyin_multi()
        .flatten()
        .flat_map(|m| m.into_iter().map(|p| pinyin_ascii(p.with_tone_num_end())))
        .collect();

      // let pinyin: String = zh.as_str()
      //   .to_pinyin()
      //   .flat_map(|p| p.map(|p| p.with_tone_num_end()))
      //   .collect();

      if pinyins.is_empty() {
        return None;
      }

      let h = match cqkm.get(&zh) {
        None => Hanzi {
          zh,
          pinyins,
          cj5,
          cqkm_form: None,
          cqkm_initials: vec![],
        },
        Some(cqkm) => Hanzi {
          zh,
          pinyins,
          cj5,
          cqkm_form: Some(cqkm.code.clone()),
          cqkm_initials: cqkm.initials.clone().into_iter().flatten().collect(),
        },
      };

      Some(h)
    })
    .collect();

  hanzis.sort();
  Ok(hanzis)
}

fn getZh<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
  let s = read_to_string(path)?;
  let value: Value = serde_json::from_str(&s)?;
  let v = match value {
    Value::Array(items) => items
      .into_iter()
      .map(|item| item["zh"].to_string())
      .collect(),
    _ => vec![],
  };

  Ok(v)
}
