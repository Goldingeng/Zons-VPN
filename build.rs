fn main() {
    let cfg = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("ui/app.slint", cfg).expect("slint");
    embed_resource::compile("assets/app.rc", embed_resource::NONE).manifest_required().unwrap();
}
