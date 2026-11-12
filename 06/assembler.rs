use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn valid_symbol(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || "_.$:".contains(c) => (),
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || "_.$:".contains(c))
}

fn comp(s: &str) -> Option<u16> {
    Some(match s {
        "0" => 0b0101010,
        "1" => 0b0111111,
        "-1" => 0b0111010,
        "D" => 0b0001100,
        "A" => 0b0110000,
        "M" => 0b1110000,
        "!D" => 0b0001101,
        "!A" => 0b0110001,
        "!M" => 0b1110001,
        "-D" => 0b0001111,
        "-A" => 0b0110011,
        "-M" => 0b1110011,
        "D+1" => 0b0011111,
        "A+1" => 0b0110111,
        "M+1" => 0b1110111,
        "D-1" => 0b0001110,
        "A-1" => 0b0110010,
        "M-1" => 0b1110010,
        "D+A" => 0b0000010,
        "D+M" => 0b1000010,
        "D-A" => 0b0010011,
        "D-M" => 0b1010011,
        "A-D" => 0b0000111,
        "M-D" => 0b1000111,
        "D&A" => 0b0000000,
        "D&M" => 0b1000000,
        "D|A" => 0b0010101,
        "D|M" => 0b1010101,
        _ => return None,
    })
}

fn c_instruction(line: &str) -> Result<u16, &'static str> {
    let (dest, rest) = match line.split_once('=') {
        Some(("", _)) => return Err("目的欄位不能為空"),
        Some(parts) => parts,
        None => ("", line),
    };
    let (operation, jump) = match rest.split_once(';') {
        Some((_, "")) => return Err("跳躍欄位不能為空"),
        Some(parts) => parts,
        None => (rest, ""),
    };
    let mut dest_bits = 0;
    for ch in dest.chars() {
        let bit = match ch {
            'A' => 4,
            'D' => 2,
            'M' => 1,
            _ => return Err("無效的目的欄位"),
        };
        if dest_bits & bit != 0 {
            return Err("重複的目的欄位");
        }
        dest_bits |= bit;
    }
    let jump_bits = match jump {
        "" => 0,
        "JGT" => 1,
        "JEQ" => 2,
        "JGE" => 3,
        "JLT" => 4,
        "JNE" => 5,
        "JLE" => 6,
        "JMP" => 7,
        _ => return Err("無效的跳躍欄位"),
    };
    let comp_bits = comp(operation).ok_or("無效的運算欄位")?;
    Ok(0b1110_0000_0000_0000 | (comp_bits << 6) | (dest_bits << 3) | jump_bits)
}

fn assemble(source: &str) -> Result<String, String> {
    let mut symbols = HashMap::new();
    for i in 0..16 {
        symbols.insert(format!("R{i}"), i);
    }
    for (name, address) in [
        ("SP", 0),
        ("LCL", 1),
        ("ARG", 2),
        ("THIS", 3),
        ("THAT", 4),
        ("SCREEN", 16384),
        ("KBD", 24576),
    ] {
        symbols.insert(name.to_owned(), address);
    }

    let mut instructions = Vec::new();
    for (index, raw) in source.trim_start_matches('\u{feff}').lines().enumerate() {
        let line: String = raw
            .split("//")
            .next()
            .unwrap()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('(') && line.ends_with(')') {
            let label = &line[1..line.len() - 1];
            if !valid_symbol(label) || symbols.contains_key(label) {
                return Err(format!("第 {} 行：無效或重複的標籤", index + 1));
            }
            if instructions.len() > 32767 {
                return Err(format!("第 {} 行：標籤位址超出範圍", index + 1));
            }
            symbols.insert(label.to_owned(), instructions.len() as u16);
        } else {
            if instructions.len() >= 32768 {
                return Err(format!("第 {} 行：程式超過 ROM 容量", index + 1));
            }
            instructions.push((index + 1, line));
        }
    }

    let mut next_variable = 16u16;
    let mut output = String::new();
    for (line_number, line) in instructions {
        let instruction = if let Some(value) = line.strip_prefix('@') {
            if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                value
                    .parse::<u16>()
                    .ok()
                    .filter(|n| *n <= 32767)
                    .ok_or_else(|| format!("第 {line_number} 行：位址必須介於 0 和 32767"))?
            } else if valid_symbol(value) {
                if !symbols.contains_key(value) {
                    if next_variable >= 16384 {
                        return Err(format!("第 {line_number} 行：變數位址超出範圍"));
                    }
                    symbols.insert(value.to_owned(), next_variable);
                    next_variable += 1;
                }
                symbols[value]
            } else {
                return Err(format!("第 {line_number} 行：無效的位址或符號"));
            }
        } else {
            c_instruction(&line).map_err(|e| format!("第 {line_number} 行：{e}"))?
        };
        output.push_str(&format!("{instruction:016b}\n"));
    }
    Ok(output)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 2 {
        return Err("用法：assembler 程式.asm".into());
    }
    let path = Path::new(&args[1]);
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("asm"))
    {
        return Err("請指定 .asm 檔案".into());
    }
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let output = assemble(&source)?;
    fs::write(path.with_extension("hack"), output).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
