use petgraph::{
    dot::{Config, Dot},
    Graph,
};
use std::{
    fs::File,
    io::{self, Error, Write},
};

pub fn write_dot<T, E>(graph: &Graph<T, E>, filename: String) -> Result<(), Error>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    if let Some(basename) = filename.split(".").next() {
        let gr = Dot::new(&graph);
        let mut f = File::create(basename.to_string() + ".dot").unwrap();
        write!(&mut f, "{:?}", gr)?;
        Ok(())
    } else {
        Err(Error::new(
            io::ErrorKind::InvalidInput,
            "Could not modify filename",
        ))
    }
}
