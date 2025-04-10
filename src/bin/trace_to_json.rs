use clap::Parser;
use clap_verbosity_flag::Verbosity;
use lib::output::make_file;
use std::io::Write;

use lib::model::ReadResult;
use lib::parser::read_file;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct ConvertCli {
    /// Input file path
    pub input: String,
    /// Output file path
    pub output: String,
    #[command(flatten)]
    pub verbosity: Verbosity,
}

fn main() {
    // Parse CLI arguments
    let cli = ConvertCli::parse();

    // Read the ExecutionGraph from the input file
    let res: ReadResult = read_file(cli.input);

    let str = serde_json::to_string(&res).unwrap();
    let mut file = make_file(cli.output).expect("Unable to create file");
    file.write_all(str.as_bytes())
        .expect("Unable to write data");
}
