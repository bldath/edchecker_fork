use std::{collections::HashMap, error::Error};

use clap::{Parser, ValueEnum};
use clap_verbosity_flag::Verbosity;
use glob::glob;
use itertools::Itertools;
use lib::{cli::ADT, heuristics::Heuristic};
use serde::Serialize;

struct ExecutionResult {
    events: u64,
    messages: u64,
    handlers: u64,
    time: Option<u64>,
    valid: Option<bool>,
}

impl ExecutionResult {
    pub fn parse(file_name: &str) -> Result<ExecutionResult, Box<dyn Error>> {
        let f = std::fs::read_to_string(file_name)?;
        let lines: Vec<&str> = f.lines().collect_vec();

        let patterns = [
            "Handlers",
            "Messages",
            "Events",
            "Parsing",
            "Preprocessing",
            "Check",
            "Total",
            "Result",
        ];
        let mut res = vec![];

        let mut lctr = 0;
        for p in patterns.iter() {
            while lctr < lines.len() && !lines[lctr].starts_with(p) {
                lctr += 1
            }

            if lctr < lines.len() {
                res.push(lines[lctr]);
                lctr += 1
            }
        }
        let [handlers, messages, events] = &res.as_slice()[0..3] else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File does not contain Expt data",
            )));
        };

        let handlers: u64 = handlers
            .chars()
            .filter(|x| x.is_ascii_digit())
            .join("")
            .parse()?;
        let messages: u64 = messages
            .chars()
            .filter(|x| x.is_ascii_digit())
            .join("")
            .parse()?;
        let events = events
            .chars()
            .filter(|x| x.is_ascii_digit())
            .join("")
            .parse()?;

        let (time, valid) = if let [total, valid] = &res.as_slice()[6..] {
        let time = total
            .chars()
            .filter(|x| x.is_ascii_digit())
            .join("")
            .parse()?;
        let valid = valid
            .split(":")
            .nth(1)
            .unwrap()
            .chars()
            .skip(1)
            .join("")
            .parse()?;

        (Some(time), Some(valid))

        } else {
            (None, None)
        };

        Ok(ExecutionResult {
            events,
            messages,
            handlers,
            time,
            valid,
        })
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
struct ExptData {
    test: String,
    param: Option<u64>,
    alg: String,
    adt: ADT,
    heur: Heuristic,
}

impl TryFrom<&str> for ExptData {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Example: ./path/to/directory/expt_param/trace3_z3_multiset_no.out
        let slashes = value.match_indices("/").collect_vec();
        let (my_idx, _) = slashes[slashes.len() - 2]; // Split at second to last slash

        let (_, core) = value.split_at(my_idx + 1);
        let &[a, b] = core.split('/').collect_vec().as_slice() else {
            return Err("Filename has wrong format".to_string());
        };

        let (expt, param) = if let &[expt, param] = a.split('_').collect_vec().as_slice() {
            (expt, Some(param))
        } else {
            (a, None)
        };

        let &[_trace, alg, adt, heur] = b
            .split('.')
            .next()
            .unwrap()
            .split('_')
            .collect_vec()
            .as_slice()
        else {
            return Err("Filename has wrong format".to_string());
        };

        let test = expt.to_string();
        let param: Option<u64> = param.and_then(|x| x.parse().ok());
        let alg = alg.to_string();
        let adt = ADT::from_str(adt, true)?;
        let heur = Heuristic::from_str(heur, true)?;

        Ok(ExptData {
            test,
            param,
            alg,
            adt,
            heur,
        })
    }
}

#[derive(Debug, Default, Serialize)]
struct ResultData {
    events: u64,
    messages: u64,
    handlers: u64,
    num_traces: u64,
    num_ok: u64,
    num_timeout: u64,
    times: Vec<u64>,
}

impl ResultData {
    pub fn add(&mut self, er: &Option<ExecutionResult>) {
        match er {
            Some(er) => self.add_result(er),
            None => self.add_fail(),
        }
        self.num_traces += 1;
    }

    fn add_result(&mut self, er: &ExecutionResult) {
        if self.events == 0 {
            self.events = er.events
        }
        if self.messages == 0 {
            self.messages = er.messages
        }
        if self.handlers == 0 {
            self.handlers = er.handlers
        }

        if let (Some(vld), Some(time)) = (er.valid, er.time) {
            if vld {
                self.num_ok += 1;
            }
            self.times.push(time);
        } else {
            self.num_timeout += 1;
        }
    }

    fn add_fail(&mut self) {
        println!("Failed to parse file, assume it timed out while parsing..");
        self.num_timeout += 1;
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct ConvertCli {
    pub input_dir: String,
    pub output_file: String,

    #[command(flatten)]
    pub verbosity: Verbosity,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = ConvertCli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    let mut results: HashMap<ExptData, ResultData> = HashMap::new();

    let inputs = glob(format!("{}/**/*.out", cli.input_dir).as_str())
        .expect("Failed to read input directory");

    for e in inputs.flatten() {
        let path = e.as_path().to_str().unwrap();
        let expt = ExptData::try_from(path)?;
        println!("Expt: {:?}", expt);

        let res = ExecutionResult::parse(path).ok();
        results.entry(expt).or_default().add(&res);
    }

    let rows = results
        .iter()
        .map(|(expt, data)| {
            let sum: u64 = data.times.iter().sum();
            let avg = (sum as f64) / (1000000. * data.times.len() as f64);
            let s = format!("{:.4}", avg);
            (
                expt.test.clone(),
                expt.param,
                data.events,
                data.messages,
                data.handlers,
                expt.alg.clone(),
                expt.adt,
                expt.heur,
                data.num_traces,
                data.num_ok,
                data.num_timeout,
                s,
            )
        })
        .collect_vec();

    let mut serializer = csv::Writer::from_path(&cli.output_file)?;

    serializer.write_record([
        "Experiment",
        "Parameter",
        "Events",
        "Messages",
        "Handlers",
        "Algorithm",
        "ADT",
        "Heuristic",
        "#Traces",
        "#OK",
        "#T/O",
        "Avg. Time",
    ])?;

    for r in rows {
        serializer.serialize(r)?;
    }

    Ok(())
}
