use std::{
  cmp::Ordering,
  collections::HashMap,
  fs::{self, OpenOptions, read_to_string},
  io::{self, BufWriter, Write},
};

use pinyin::ToPinyinMulti;
use serde::{Deserialize, Serialize};

fn main() -> io::Result<()> {
  let res = get_hanzis(
    &["json/cqkm-char.json", "json/cqkm-char-extra.json"],
    &["yong/cj5-20902.txt"],
  );

  if let Err(e) = &res {
    println!("{}", e)
  }
  let mut hanzis = res.unwrap();
  dbg!(hanzis.len());
  let cqkm_count = hanzis.iter().filter(|z| z.cqkm_form.is_some()).count();
  dbg!(cqkm_count);
  hanzis.iter_mut().for_each(|z| z.fill_cqkm_initials());

  for key in "qwertyuiopasdfghjkl;zxc.b,mnv".split("") {
    let found = hanzis.iter().find(|z| z.cj5.iter().any(|s| s == key));
    if let Some(z) = found {
      println!("{}: {}", key, z.zh);
    }
  }

  let mut codes: Vec<Code> = hanzis.iter().flat_map(|z| z.codes()).collect();

  let s = read_to_string("json/cqkm-char-shortcuts.json")?;
  let shortcuts: Vec<CqkmWord> = serde_json::from_str(&s)?;
  dbg!(shortcuts.len());

  let xhs = XHS.into_iter().collect::<HashMap<&str, &str>>();
  let hanzim: HashMap<&String, &Hanzi> = hanzis.iter().map(|z| (&z.zh, z)).collect();

  for short in shortcuts {
    let initial = &short.spell[0..1];
    let code = if "zcs".contains(initial) {
      let xh = format!("{initial}h");
      if hanzim
        .get(&short.zh)
        .map(|z| z.pinyins.iter().any(|p| p.starts_with(&xh)))
        .unwrap_or(false)
      {
        format!("{}{}", xhs.get(xh.as_str()).unwrap(), &short.spell[1..])
      } else {
        short.spell
      }
    } else {
      short.spell
    };

    codes.push(Code {
      zh: short.zh,
      code,
      schema: "cqkm".to_string(),
      nth: 0,
    });
  }

  codes.sort();

  json_array_write("json/cqkm-cj5-21000.json", &hanzis)?;
  json_array_write("json/zi-spells-21000.json", &codes)?;

  Ok(())
}

const XHS: [(&str, &str); 3] = [("zh", "i"), ("ch", "o"), ("sh", "u")];
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

  fn fill_cqkm_initials(&mut self) {
    let xhs = XHS.into_iter().collect::<HashMap<&str, &str>>();

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

    let replace_xhs = |initial: &&str| xhs.get(initial).unwrap_or(initial).to_string();

    if self.cqkm_initials.is_empty() {
      self.cqkm_initials.append(
        &mut pinyin_initials
          .into_iter()
          .map(|s| replace_xhs(&s))
          .collect(),
      );
    } else {
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
  paths_cj5_yong: &[&'static str],
) -> Result<Vec<Hanzi>, Box<dyn std::error::Error>> {
  let mut chars_cqkm: Vec<CqkmChar> = vec![];
  for path in paths_cqkm_char {
    let s = fs::read_to_string(path)?;
    let mut chars: Vec<CqkmChar> = serde_json::from_str(&s)?;
    chars_cqkm.append(&mut chars);
  }
  let cqkm: HashMap<String, CqkmChar> = chars_cqkm.into_iter().map(|c| (c.zh.clone(), c)).collect();

  let mut cj5def: HashMap<String, Vec<String>> = HashMap::new();
  for path in paths_cj5_yong {
    let s = fs::read_to_string(path)?;
    for line in s.lines() {
      let seps: Vec<&str> = line.split_whitespace().collect();
      if seps.len() == 2
        && seps[0].chars().all(|c| c.is_ascii_alphabetic())
        && seps[1].chars().count() == 1
        && seps[1].chars().all(|c| c.is_alphabetic())
      {
        let spell = seps[0].to_string();
        let zh = seps[1].to_string();
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

      if pinyins.is_empty() {
        return None
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
