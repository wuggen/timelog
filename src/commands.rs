use crate::timelog::TimeLog;

#[derive(Debug, Clone, StructOpt)]
pub enum Command {
    Open {
        tag: Option<String>,
    },

    Close {
        tag: Option<String>,
    },

    List,
}

impl Command {
    pub fn execute(&self, timelog: &mut TimeLog) {
        unimplemented!()
    }
}
