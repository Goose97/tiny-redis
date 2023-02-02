use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
struct CommandBenchResult(String, f32, f32);

fn main() {
    let commands = get_ready_commands();
    let redis_bench_result = run_benchmark(&commands, 6379);
    let our_bench_result = run_benchmark(&commands, 7878);

    let bench_result: Vec<_> = commands
        .into_iter()
        .filter_map(|command| {
            let redis_ops = redis_bench_result.get(&command);
            let our_ops = our_bench_result.get(&command);

            if redis_ops.is_some() && our_ops.is_some() {
                Some(CommandBenchResult(
                    command,
                    redis_ops.unwrap().clone(),
                    our_ops.unwrap().clone(),
                ))
            } else {
                None
            }
        })
        .collect();

    // Print result as Markdown table
    let headers = format!(
        "| Command | redis (op/s) | tiny_redis (op/s) | Comparison |\n| --- | --- | --- | --- |"
    );
    println!("{}", headers);

    for CommandBenchResult(command, redis, our) in bench_result {
        let comparison = if redis > our {
            format!("❌ -{:.2}%", (redis - our) * 100.0 / redis)
        } else {
            format!("✅ +{:.2}%", (our - redis) * 100.0 / redis)
        };
        println!("| {command:} | {redis:} | {our:} | {comparison:} |");
    }
}

fn run_benchmark(commands: &Vec<String>, port: i32) -> HashMap<String, f32> {
    let shell_command = format!(
        "redis-benchmark -h localhost -p {} -c 100 -n 100000 -k 1 -t {} --csv",
        port,
        commands.join(",")
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .expect("failed to execute process");

    let output = String::from_utf8(output.stdout).unwrap();
    output
        .trim()
        .split("\n")
        .map(|record| {
            let re = Regex::new(r#"^"(.+)?","([\d\.]+)"$"#).unwrap();
            let caps = re.captures(&record).unwrap();
            let command = caps[1].to_owned();
            let ops: f32 = caps[2].to_owned().parse().unwrap();
            (command, ops)
        })
        .collect()
}

fn get_ready_commands() -> Vec<String> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("README.md");

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    reader
        .lines()
        .map(|result| result.unwrap())
        .filter(|line| line.starts_with("- [x]"))
        .map(|line| {
            let re = Regex::new(r"[A-Z]+$").unwrap();
            let caps = re.captures(&line).unwrap();
            caps[0].to_owned()
        })
        .collect()
}
