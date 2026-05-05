use clap::{Parser};
use clap_verbosity_flag::Verbosity;
use lib::{model::mk_graph, output::write_graph, parser::read_file};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct RwCli {
    pub file: String,
    #[command(flatten)]
    pub verbosity: Verbosity,
}

fn main() -> Result<(), std::io::Error> {
    let cli = RwCli::parse();

    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    let q = read_file(cli.file.clone());

    let q = mk_graph(&q);

    write_graph(
        &q,
        cli.file.split(".").next().unwrap().to_string() + "_copy.trace",
    )
}
