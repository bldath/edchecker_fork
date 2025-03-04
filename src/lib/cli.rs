use clap::{Parser, ValueEnum};
use clap_verbosity_flag::Verbosity;
use crate::heuristics::Heuristic;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ADT {
    Multiset,
    Queue,
    Stack,
    Register
}


#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    /// Input file
    pub file: String,
    /// ADT to check consistency for
    #[arg(value_enum)]
    pub adt: ADT,

    #[arg(value_enum)]
    pub heuristics: Heuristic,

    /// Print output graphs to dotfiles with name <FILE>.dot and <FILE>_ok.dot if check succeeds.
    #[arg(short, long)]
    pub draw: bool,

    /// Verbosity for more debugging output. The more -v's the more verbose. -vvvvvvvvvvvvvvvvvvvvvvvvvvvv
    #[command(flatten)]
    pub verbosity: Verbosity,
}
