// Copyright © 2022 David Caldwell <david@porkrind.org>

use std::error::Error;

#[derive(Debug, Clone)]
pub struct Ini {
    section: Vec<Section>
}

#[derive(Debug, Clone)]
struct Section {
    name: String,
    entry: Vec<Entry>,
}

#[derive(Debug, Clone)]
enum Entry {
    KV { key: String, value: String },
    Comment(String), // Includes comment character itself and also blank lines
}

impl Ini {
    pub fn read(file: &std::path::Path) -> Result<Ini, Box<dyn Error>> {
        use std::io::BufRead;
        let file = std::fs::File::open(file)?;
        let mut ini = Ini { section: vec![Section { name: "".to_string(), entry: Vec::new()}], };
        let mut section = &mut ini.section[0];

        let section_re = regex::Regex::new(r"^\s*\[([^]]+)\]\s*$").unwrap();
        let kv_re      = regex::Regex::new(r"^\s*([^=]+)\s*=\s*(.*)$").unwrap();
        let comment_re = regex::Regex::new(r"^\s*(?:;.*|)$").unwrap();
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if comment_re.is_match(&line) {
                section.entry.push(Entry::Comment(line.to_string()));
            } else if let Some(caps) = section_re.captures(&line) {
                ini.section.push(Section { name: caps.get(1).unwrap().as_str().trim().to_string(),
                                           entry: Vec::new() });
                section = ini.section.last_mut().unwrap();
            } else if let Some(caps) = kv_re.captures(&line) {
                section.entry.push(Entry::KV { key:   caps.get(1).unwrap().as_str().trim().to_string(),
                                               value: caps.get(2).unwrap().as_str().trim().to_string(), });
            } else {
                section.entry.push(Entry::Comment(line));
            }
        }
        Ok(ini)
    }

    pub fn write(&self, file: &std::path::Path) -> Result<(), Box<dyn Error>> {
        use std::io::Write;
        let mut file = std::fs::File::create(file)?;
        for s in &self.section {
            if s.name != "" {
                file.write_fmt(format_args!("[{}]\n", s.name))?;
            }
            for e in &s.entry {
                match e {
                    Entry::KV { key: k, value: v } => { file.write_fmt(format_args!("{} = {}\n", k, v))?; }
                    Entry::Comment(line)           => { file.write_fmt(format_args!("{}\n", line))?; }
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        for s in &self.section {
            if s.name == section {
                for e in &s.entry {
                    match e {
                        Entry::KV { key: k, value: v } if key == k => { return Some(&v) }
                        _ => {}
                    }
                }
            }
        }
        None
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let new = Entry::KV { key:   key.trim().to_string(),
                              value: value.trim().to_string(), };
        for s in &mut self.section {
            if s.name == section {
                for e in &mut s.entry {
                    match e {
                        Entry::KV { key: k, value: _ } if key == k => {
                            *e = new;
                            return;
                        }
                        _ => {}
                    }
                }
                // No existing entry, append it to section
                s.entry.push(new);
                return;
            }
        }
        // No existing section, append it to file and add entry
        self.section.push(Section { name: section.trim().to_string(),
                                    entry: vec![new] });
    }
}

