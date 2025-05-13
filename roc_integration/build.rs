fn main() {
    if std::env::var("CARGO_FEATURE_ROC_CODEGEN").is_ok() {
        let git = vergen_gitcl::GitclBuilder::default()
            .sha(true)
            .dirty(false)
            .build()
            .unwrap();

        vergen_gitcl::Emitter::default()
            .add_instructions(&git)
            .unwrap()
            .emit()
            .unwrap();
    }
}
