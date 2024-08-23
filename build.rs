fn main() {
    let config =
        slint_build::CompilerConfiguration::new()
        .with_style("native".into());
        // .with_style("material".into());
    slint_build::compile_with_config("src/psylink.slint", config).unwrap();
}
