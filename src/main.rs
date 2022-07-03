use impact::run;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // Returning a `Result` from `main` does not work with WASM
        fn main() {
            pollster::block_on(run::run_wasm())
        }

    } else {
        use anyhow::Result;

        fn main() -> Result<()> {
            pollster::block_on(run::run())
        }

    }
}
