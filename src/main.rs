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
        .flexible(true)
        .from_reader(reader);

    let mut store = Store::new();

    for result in rdr.deserialize() {
        let transaction: Transaction = result?;
        if let Err(e) = transaction.save(&mut store) {
            warn!("{e}");
        }
    }

    let mut wtr = Writer::from_writer(writer);

    for account in store.accounts.values() {
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

"client,available,held,total,locked
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

"client,available,held,total,locked
1,1.5000,0.0000,1.5000,false
2,2.0000,0.0000,2.0000,false
";

"handles deposits and withdrawals properly"
)]
    #[test_case(
"type, client, tx, amount
deposit, 1, 1, 1.56787645323 
deposit, 2, 2, 2.2345652
deposit, 1, 3, 2.34354 
withdrawal, 1, 4, 1.522454 
withdrawal, 2, 5, 3.0014355",

"client,available,held,total,locked
1,2.3889,0.0000,2.3889,false
2,2.2345,0.0000,2.2345,false
";

"handles at least 4 decimal places properly"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1,
deposit, 1, 1, 1.5 
withdrawal, 2, 5, 3.0", 

"client,available,held,total,locked
1,1.5000,1.0000,2.5000,false
2,2.0000,0.0000,2.0000,false
";

"handles client 1 dispute properly"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1, 2.0
resolve, 1, 1,
withdrawal, 2, 5, 3.0", 

"client,available,held,total,locked
1,1.0000,0.0000,1.0000,false
2,2.0000,0.0000,2.0000,false
";

"can successfully resolve dispute"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1,
chargeback, 1, 1,
withdrawal, 2, 5, 3.0", 

"client,available,held,total,locked
1,0.0000,0.0000,0.0000,true
2,2.0000,0.0000,2.0000,false
";

"locks account 1 when client 1 charges back"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1,
chargeback, 1, 1,
withdrawal, 2, 5, 3.0
dispute, 2, 5,
chargeback, 2, 5,", 

// I'm assuming there can be a negative balance in case of chargeback.
// Should this be prevented instead?
"client,available,held,total,locked
1,0.0000,0.0000,0.0000,true
2,-1.0000,0.0000,-1.0000,true
";

"locks accounts 1 and 2 when clients 1 and 2 initiate chargebacks"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0 
dispute, 1, 1
chargeback, 1, 1,
deposit, 1, 2, 5.6587878
deposit, 1, 3, 11.05
withdrawal, 1, 5, 3.0", 

"client,available,held,total,locked
1,0.0000,0.0000,0.0000,true
2,2.0000,0.0000,2.0000,false
";

"cannot carry out a transaction after account is locked"
)]
    #[test_case(
"type, client, tx, amount 
deposit, 1, 1, 1.0
deposit, 1, 2, -0.0001", 

"client,available,held,total,locked
1,1.0000,0.0000,1.0000,false
";

"does not accept negative amount"
)]
    fn transactions_to_accounts(input_transaction: &str, output_account: &str) {
        let mut result = Vec::new();

        generate_accounts_from_transactions(input_transaction.as_bytes(), &mut result)
            .expect("Something failed");
        assert_eq!(result, output_account.as_bytes());
    }
}
