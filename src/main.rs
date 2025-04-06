use anyhow::Result;
use impact::{run, scripting::Callbacks};

fn main() -> Result<()> {
    run::run(|_| {}, Callbacks::default())
}
