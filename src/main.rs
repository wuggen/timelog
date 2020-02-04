use serde_json;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
struct Options {
    num: Option<i32>,
}

fn main() -> serde_json::Result<()> {
    let Options { num } = Options::from_args();
    println!("{}", serde_json::to_string(&num)?);
    Ok(())
}
