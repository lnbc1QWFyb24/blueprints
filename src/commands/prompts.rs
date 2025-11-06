use anyhow::Result;
use clap::Args;

use super::common::{
    Tokens, prepare_blueprints_for_crate, resolve_target_from_crate,
    resolve_target_from_module_path,
};
use crate::prompts::builder::Profile;

#[derive(Args, Debug, Clone)]
pub struct PromptsArgs {
    /// Target Cargo package name (crate)
    #[arg(
        long = "crate",
        value_name = "PKG",
        conflicts_with = "module",
        required_unless_present = "module"
    )]
    pub krate: Option<String>,

    /// Target module path (file or dir) inside a crate
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with = "krate",
        required_unless_present = "krate"
    )]
    pub module: Option<String>,
}

pub fn handle(args: &PromptsArgs) -> Result<()> {
    let target = if let Some(name) = &args.krate {
        resolve_target_from_crate(name)?
    } else if let Some(path) = &args.module {
        resolve_target_from_module_path(path)?
    } else {
        anyhow::bail!("specify exactly one of --crate or --module");
    };
    let blueprints = prepare_blueprints_for_crate(&target)?;
    let dir_token = blueprints.dir_token_value();
    let crate_root_token = {
        let p = &target.crate_root;
        let s = p.to_string_lossy();
        if p.is_relative() && !s.starts_with("./") && !s.starts_with("../") {
            format!("./{s}")
        } else {
            s.into_owned()
        }
    };
    let module_rel_token = target.module_rel.as_ref().map(|p| {
        let s = p.to_string_lossy();
        if p.is_relative() && !s.starts_with("./") && !s.starts_with("../") {
            format!("./{s}")
        } else {
            s.into_owned()
        }
    });

    let design = Profile::Design
        .compose()
        .with_blueprints_dir(dir_token.clone())
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token.clone())
        .inline_blueprints()
        .build()?;

    let review = Profile::Review
        .compose()
        .with_blueprints_dir(dir_token.clone())
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token.clone())
        .inline_blueprints()
        .build()?;

    let update = Profile::Update
        .compose()
        .with_blueprints_dir(dir_token.clone())
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token.clone())
        .inline_blueprints()
        .build()?;

    let tokens = Tokens::new();
    let mut reviewer_builder = Profile::ImplementReviewer
        .compose()
        .with_blueprints_dir(dir_token.clone())
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token.clone())
        .inline_blueprints();
    if let Some(mrel) = &module_rel_token {
        reviewer_builder = reviewer_builder.with_variable("MODULE_REL_PATH", mrel.clone());
    }
    let implement_reviewer = tokens.apply(&reviewer_builder.build()?);

    let mut builder_builder = Profile::ImplementBuilder
        .compose()
        .with_blueprints_dir(dir_token)
        .with_variable("CRATE_NAME", target.crate_name.clone())
        .with_variable("CRATE_ROOT", crate_root_token);
    if let Some(mrel) = module_rel_token {
        builder_builder = builder_builder.with_variable("MODULE_REL_PATH", mrel);
    }
    builder_builder = builder_builder.inline_blueprints();
    let implement_builder = tokens.apply(&builder_builder.build()?);

    print_prompt("design", &design);
    print_prompt("review", &review);
    print_prompt("update", &update);
    print_prompt("implement (reviewer)", &implement_reviewer);
    print_prompt("implement (builder)", &implement_builder);

    Ok(())
}

fn print_prompt(title: &str, body: &str) {
    println!("=== {title} ===");
    print!("{body}");
    if !body.ends_with('\n') {
        println!();
    }
    println!();
}
