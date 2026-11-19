use std::fmt::Write;
use std::{env, fs, path::PathBuf, process::ExitCode};

#[derive(Default)]
struct Translator {
    asm: String,
    file: String,
    next_label: usize,
}

impl Translator {
    fn emit(&mut self, code: &str) {
        self.asm.push_str(code);
        self.asm.push('\n');
    }

    fn push(&mut self) {
        self.emit("@SP\nA=M\nM=D\n@SP\nM=M+1");
    }

    fn pop(&mut self) {
        self.emit("@SP\nAM=M-1\nD=M");
    }

    fn label(&mut self) -> String {
        self.next_label += 1;
        format!("$VM.${}", self.next_label)
    }

    fn compare(&mut self, op: &str) {
        let label = self.label();
        self.pop();
        if op != "eq" {
            self.emit("@R13\nM=D\n@SP\nA=M-1\nD=M");
            let _ = writeln!(self.asm, "@{label}.negative\nD;JLT\n@R13\nD=M\n@{label}.{}\nD;JLT\n@{label}.subtract\n0;JMP\n({label}.negative)\n@R13\nD=M\n@{label}.{}\nD;JGE\n({label}.subtract)\n@R13\nD=M",
                if op == "gt" { "yes" } else { "no" },
                if op == "lt" { "yes" } else { "no" });
        }
        self.emit("@SP\nA=M-1\nD=M-D");
        let jump = match op {
            "gt" => "JGT",
            "lt" => "JLT",
            _ => "JEQ",
        };
        let _ = writeln!(self.asm, "@{label}.yes\nD;{jump}\n({label}.no)\nD=0\n@{label}.end\n0;JMP\n({label}.yes)\nD=-1\n({label}.end)\n@SP\nA=M-1\nM=D");
    }

    fn memory(&mut self, push: bool, segment: &str, index: u16) -> Result<(), &'static str> {
        let base = match segment {
            "local" => Some("LCL"),
            "argument" => Some("ARG"),
            "this" => Some("THIS"),
            "that" => Some("THAT"),
            _ => None,
        };
        if segment == "constant" && push {
            let _ = writeln!(self.asm, "@{index}\nD=A");
            self.push();
        } else if let Some(base) = base {
            let _ = writeln!(self.asm, "@{index}\nD=A\n@{base}");
            if push {
                self.emit("A=D+M\nD=M");
                self.push();
            } else {
                self.emit("D=D+M\n@R13\nM=D");
                self.pop();
                self.emit("@R13\nA=M\nM=D");
            }
        } else {
            let address = match segment {
                "temp" if index < 8 => (5 + index).to_string(),
                "pointer" if index < 2 => (3 + index).to_string(),
                "static" if index < 240 => format!("{}.{index}", self.file),
                _ => return Err("無效的記憶體區段或索引"),
            };
            if push {
                let _ = writeln!(self.asm, "@{address}\nD=M");
                self.push();
            } else {
                self.pop();
                let _ = writeln!(self.asm, "@{address}\nM=D");
            }
        }
        Ok(())
    }

    fn command(&mut self, command: &[&str]) -> Result<(), &'static str> {
        match command {
            [op @ ("push" | "pop"), segment, index] => {
                self.memory(*op == "push", segment, number(index)?)?
            }
            [op @ ("add" | "sub" | "and" | "or")] => {
                self.pop();
                self.emit("@SP\nA=M-1");
                self.emit(match *op {
                    "add" => "M=D+M",
                    "sub" => "M=M-D",
                    "and" => "M=D&M",
                    _ => "M=D|M",
                });
            }
            ["neg"] => self.emit("@SP\nA=M-1\nM=-M"),
            ["not"] => self.emit("@SP\nA=M-1\nM=!M"),
            [op @ ("eq" | "gt" | "lt")] => self.compare(op),
            _ => return Err("無效的 VM 指令"),
        }
        Ok(())
    }
}

fn number(s: &str) -> Result<u16, &'static str> {
    s.parse::<u16>()
        .ok()
        .filter(|&n| n <= 32767 && s.bytes().all(|c| c.is_ascii_digit()))
        .ok_or("索引必須介於 0 和 32767")
}

fn symbol(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with("$VM.")
        && s.bytes().enumerate().all(|(i, c)| {
            c.is_ascii_alphabetic() || b"_.$:".contains(&c) || (i > 0 && c.is_ascii_digit())
        })
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let usage = "用法：vm_translator 檔案.vm";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    if args.next().is_some() || !input.extension().is_some_and(|ext| ext == "vm") {
        return Err(usage.into());
    }
    let name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| symbol(s))
        .ok_or("無效的 VM 檔名")?;
    let mut vm = Translator {
        file: name.to_owned(),
        ..Translator::default()
    };
    let source = fs::read_to_string(&input)?;
    for (i, raw) in source.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = raw.split_once("//").map_or(raw, |(code, _)| code);
        let words: Vec<_> = line.split_whitespace().collect();
        if !words.is_empty() {
            vm.command(&words)
                .map_err(|e| format!("{}:{}: {e}", input.display(), i + 1))?;
        }
    }
    fs::write(input.with_extension("asm"), vm.asm)?;
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("{e}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
