use std::process;
#[macro_use]
extern crate log;
// use log::{info, trace, warn};

mod features;
use features::{Store, Transaction};

fn main() {
    env_logger::init();
    if run().is_err() {
        process::exit(1);
    }
}

fn run() -> anyhow::Result<Store> {
    let data = "\
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
withdrawal, 2, 5, 3.0
withdrawal, 2, 5, 3.0
dispute, 2, 5, 1.0
dispute, 2, 5, 1.0
";

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(data.as_bytes());

    let mut store = Store::new();

    for result in rdr.deserialize() {
        let transaction: Transaction = result?;
        if let Err(e) = transaction.record(&mut store) {
            warn!("{e}");
        }
    }

    Ok(store)
}
