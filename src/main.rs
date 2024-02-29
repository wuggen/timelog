use timelog::commands::{CommandError, StdOutputs};
use timelog::config::{self, ConfigError, Options};

use structopt::StructOpt;

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), MainError> {
    let options = Options::from_args();

    stderrlog::new().verbosity(options.verbose).init().unwrap();

    let mut timelog = config::current_timelog(&options)?;
    let outputs = StdOutputs::default();
    if options.command.execute(&mut timelog, outputs)?.is_changed() {
        config::write_timelog(&options, &timelog)?;
    }
    Ok(())
}

#[derive(Debug)]
enum MainError {
    ConfigError(ConfigError),
    CommandError(CommandError),
}

impl Display for MainError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MainError::ConfigError(err) => write!(f, "{}", err),
            MainError::CommandError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for MainError {}

impl From<ConfigError> for MainError {
    fn from(err: ConfigError) -> MainError {
        MainError::ConfigError(err)
    }
}

impl From<CommandError> for MainError {
    fn from(err: CommandError) -> MainError {
        MainError::CommandError(err)
    }
}
