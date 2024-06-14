use anyhow::Result;
use impact::run;

fn main() -> Result<()> {
    pollster::block_on(run::run())
}
