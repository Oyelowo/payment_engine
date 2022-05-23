use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    process,
};
#[macro_use]
extern crate log;

mod features;
use csv::Writer;
use features::{Store, Transaction};

fn main() {
    env_logger::init();

    let args = env::args().into_iter().collect::<Vec<_>>();
    let transactions_file_name = &args[1];

    let f = File::open(transactions_file_name).expect("Unable to open file");
    let reader = BufReader::new(f);

    if generate_accounts_from_transactions(reader, io::stdout()).is_err() {
        process::exit(1);
    }
}

fn generate_accounts_from_transactions(
    reader: impl BufRead,
    writer: impl Write,
) -> anyhow::Result<()> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .trim(csv::Trim::All)
        .from_reader(reader);

    let mut store = Store::new();

    for result in rdr.deserialize() {
        let transaction: Transaction = result?;
        if let Err(e) = transaction.save(&mut store) {
            warn!("{e}");
        }
    }

    let mut wtr = Writer::from_writer(writer);

    for account in store.client_accounts.values() {
        wtr.serialize(account)?;
    }
    wtr.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0 
withdrawal, 1, 4, 1.5",

"client_id,available,held,total,locked
1,1.0000,0.0000,1.0000,false
";

"cannot withdraw more than available"
)]
    #[test_case(
"type, client, tx, amount
deposit, 1, 1, 1.0 
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0 
withdrawal, 1, 4, 1.5 
withdrawal, 2, 5, 3.0",

"client_id,available,held,total,locked
1,1.5000,0.0000,1.5000,false
2,2.0000,0.0000,2.0000,false
";

"when deposits and withdrawals only"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1, 2.0 
deposit, 1, 1, 1.5 
withdrawal, 2, 5, 3.0", 

"client_id,available,held,total,locked
1,1.5000,1.0000,2.5000,false
2,2.0000,0.0000,2.0000,false
";

"when client 1 disputes"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1, 2.0 
resolve, 1, 1,
withdrawal, 2, 5, 3.0", 

"client_id,available,held,total,locked
1,1.0000,0.0000,1.0000,false
2,2.0000,0.0000,2.0000,false
";

"successfully resolves disputed"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1, 2.0 
chargeback, 1, 1, 1.5 
withdrawal, 2, 5, 3.0", 

"client_id,available,held,total,locked
1,0.0000,0.0000,0.0000,true
2,2.0000,0.0000,2.0000,false
";

"locks account 1 when client 1 charges back"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1, 2.0 
chargeback, 1, 1, 1.5 
withdrawal, 2, 5, 3.0
dispute, 2, 5, 3.0
chargeback, 2, 5, 3.0", 

// I'm assuming there can be a negative balance in case of chargeback.
// Should this be prevented instead?
"client_id,available,held,total,locked
1,0.0000,0.0000,0.0000,true
2,-1.0000,0.0000,-1.0000,true
";

"locks accounts 1 and 2 when clients 1 and 2 inittiate chargeback"
)]
    fn transactions_to_accounts(input_transaction: &str, output_account: &str) {
        let mut result = Vec::new();

        generate_accounts_from_transactions(input_transaction.as_bytes(), &mut result)
            .expect("Something failed");
        assert_eq!(result, output_account.as_bytes());
    }
}
