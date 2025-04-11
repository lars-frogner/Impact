use anyhow::Result;
use impact::{
    application::{Application, ApplicationConfig},
    run,
    scripting::Callbacks,
};
use std::sync::{Arc, RwLock};

static APP: RwLock<Option<Arc<Application>>> = RwLock::new(None);

pub fn run() -> Result<()> {
    run::run(
        ApplicationConfig::default(),
        |app| {
            *APP.write().unwrap() = Some(app);
        },
        Callbacks::default(),
    )
}
