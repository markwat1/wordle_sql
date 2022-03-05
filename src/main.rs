use regex::Regex;
use rusqlite::{Connection, Result};
use std::{collections::HashMap, env};

const MISS: u8 = 0;
const BLOW: u8 = 1;
const HIT: u8 = 2;

#[test]
fn t_check_wordle() {
    let r = check_wordle(&"test".to_string(), &"test2".to_string());
    let answer: Vec<u8> = vec![MISS, MISS, MISS, MISS];
    assert_eq!(r, answer);
    let r = check_wordle(&"test2".to_string(), &"test2".to_string());
    let answer: Vec<u8> = vec![HIT, HIT, HIT, HIT, HIT];
    assert_eq!(r, answer);
    let r = check_wordle(&"abcde".to_string(), &"etaio".to_string());
    let answer: Vec<u8> = vec![BLOW, MISS, MISS, MISS, BLOW];
    assert_eq!(r, answer);
    let r = check_wordle(&"raise".to_string(), &"roops".to_string());
    let answer: Vec<u8> = vec![HIT, MISS, MISS, BLOW, MISS];
    assert_eq!(r, answer);
    let r = check_wordle(&"raise".to_string(), &"cynic".to_string());
    let answer: Vec<u8> = vec![MISS, MISS, BLOW, MISS, MISS];
    assert_eq!(r, answer);
    let r = check_wordle(&"indol".to_string(), &"cynic".to_string());
    let answer: Vec<u8> = vec![BLOW, BLOW, MISS, MISS, MISS];
    assert_eq!(r, answer);
    let r = check_wordle(&"cutin".to_string(), &"cynic".to_string());
    let answer: Vec<u8> = vec![HIT, MISS, MISS, HIT, BLOW];
    assert_eq!(r, answer);
    let r = check_wordle(&"civic".to_string(), &"cynic".to_string());
    let answer: Vec<u8> = vec![HIT, MISS, MISS, HIT, HIT];
    assert_eq!(r, answer);
    let r = check_wordle(&"shining".to_string(), &"singing".to_string());
    let answer: Vec<u8> = vec![HIT, MISS, BLOW, BLOW, HIT, HIT, HIT];
    assert_eq!(r, answer);
}

///
/// calculate check wordle result
///
fn check_wordle(guess: &String, word: &String) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(guess.len());
    for _i in 0..guess.len() {
        result.push(MISS);
    }
    assert_eq!(result.len(), guess.len());
    if guess.len() == word.len() {
        // check HIT
        for (i, c) in guess.chars().enumerate() {
            if word.chars().nth(i).unwrap() == c {
                // HIT
                result[i] = HIT;
            }
        }
        // check BLOW
        for (i, c) in guess.chars().enumerate() {
            if result[i] != HIT {
                for (t, w) in word.chars().enumerate() {
                    if w == c && i != t && result[i] == MISS && result[t] != HIT {
                        result[i] = BLOW;
                    }
                }
            }
        }
    }
    result
}

fn minimum_weight(weights: &HashMap<String, u64>) -> String {
    let mut min: u64 = 100000000000;
    let mut min_word = String::new();
    for (k, v) in weights {
        if *v < min {
            min = *v;
            min_word = k.to_string();
        }
    }
    min_word
}

fn maximum_weight(weights: &HashMap<String, u64>) -> String {
    let mut max: u64 = 0;
    let mut max_word = String::new();
    for (k, v) in weights {
        if *v > max {
            max = *v;
            max_word = k.to_string();
        }
    }
    max_word
}

fn match_result(result: Vec<u8>, r: &String) -> bool {
    let mut pos = 0;
    for c in r.chars() {
        if result[pos] != c as u8 - '0' as u8 {
            return false;
        }
        pos += 1;
    }
    true
}

fn connect_db(db_name: String) -> Connection {
    let dbcon = match Connection::open(db_name) {
        Ok(c) => c,
        Err(e) => {
            panic!("DB Error {}", e);
        }
    };
    dbcon
}

fn delete_words(dbcon: &Connection, words: &Vec<String>) {
    let mut wordlist = "".to_string();
    let mut c = 0;
    for w in words {
        if c > 0 {
            wordlist = format!("{},'{}'", wordlist, w);
        } else {
            wordlist = format!("'{}'", w);
        }
        c += 1;
    }
    println!("wordlist:{}", wordlist);
    let st = format!("delete from word_weight where word in ({});", wordlist);
    match dbcon.execute(&st, []) {
        Err(e) => panic!("execute {}", e),
        _ => (),
    };
}

fn get_word_weight(dbcon: &Connection) -> Result<HashMap<String, u64>> {
    let mut weight_list = HashMap::new();
    let mut statement = dbcon.prepare("select * from word_weight;")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        weight_list.insert(row.get(1).unwrap(), row.get(2).unwrap());
    }
    Ok(weight_list)
}

fn get_candidate(
    word_weight: &HashMap<String, u64>,
    result_list: &Vec<(String, String)>,
) -> HashMap<String, u64> {
    let mut candidate: HashMap<String, u64> = HashMap::new();
    if result_list.len() == 0 {
        for h in word_weight {
            candidate.insert(h.0.clone(), *h.1);
        }
    } else {
        for h in word_weight.iter() {
            let mut ok = true;
            for l in result_list {
                let r = check_wordle(&l.0, h.0);
                if match_result(r, &l.1) == false {
                    ok = false;
                }
            }
            if ok {
                candidate.insert(h.0.clone(), *h.1);
            }
        }
    }
    candidate
}

fn main() {
    let db_name: String = "Words".to_string();
    let db_extention: String = ".db".to_string();
    let args: Vec<String> = env::args().collect();
    let mut result_list: Vec<(String, String)> = Vec::new();
    let mut exclude_list: Vec<String> = Vec::new();
    let mut length: usize = 5;
    if args.len() > 1 {
        let result_pattern = Regex::new(r"([a-z]+):([0-2]+)").unwrap();
        let exclude_pattern = Regex::new(r"^-([a-z]+)$").unwrap();
        let option_pattern = Regex::new(r"^-l([0-9]+)$").unwrap();
        for r in args {
            let mut skip = false;
            for cap in result_pattern.captures_iter(&r) {
                result_list.push((cap[1].to_string(), cap[2].to_string()));
                skip = true;
            }
            if skip {
                continue;
            };
            for cap in exclude_pattern.captures_iter(&r) {
                exclude_list.push(cap[1].to_string());
                skip = true;
            }
            if skip {
                continue;
            };
            for cap in option_pattern.captures_iter(&r) {
                length = match cap[1].parse::<usize>() {
                    Ok(v) => v,
                    Err(_) => return,
                }
            }
        }
    }
    let db_filename = format!("{}{}{}", db_name, length, db_extention);
    println!("DB file {}", db_filename);
    let dbcon = connect_db(db_filename);
    delete_words(&dbcon, &exclude_list);
    let word_weight = match get_word_weight(&dbcon) {
        Ok(w) => w,
        Err(e) => panic!("get_word_weight {}", e),
    };
    let candidate = get_candidate(&word_weight, &result_list);
    if candidate.len() > 0 {
        println!("candidate {}", candidate.len());
        let min = minimum_weight(&candidate);
        println!(
            "Minimum word : {} = {}",
            min,
            candidate.get(&min).expect("Notfound")
        );
        let max = maximum_weight(&candidate);
        println!(
            "Maximum word : {} = {}",
            max,
            candidate.get(&max).expect("Notfound")
        );
    } else {
        println!("No words matches");
    }
}
