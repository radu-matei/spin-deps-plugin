use anyhow::Result;
use clap::{Args, ValueEnum};
use std::path::PathBuf;
use tokio::fs;
use wit_parser::Resolve;

use crate::common::constants::{SPIN_DEPS_WIT_FILE_NAME, SPIN_WIT_DIRECTORY};

#[derive(Debug, Clone, ValueEnum)]
pub enum BindingsLanguage {
    Ts,
    Rust,
}

#[derive(Args, Debug)]
pub struct GenerateBindingsCommand {
    /// The programming language to generate bindings in
    #[clap(short = 'L', long)]
    pub language: BindingsLanguage,

    /// Output directory
    #[clap(short = 'o', long)]
    pub output: PathBuf,

    /// Id of the component, which dependencies to generate the bindings for
    #[clap(short = 'c', long)]
    pub component_id: String,
}

impl GenerateBindingsCommand {
    pub async fn run(&self) -> Result<()> {
        let wit_path = PathBuf::from(SPIN_WIT_DIRECTORY)
            .join(&self.component_id)
            .join(SPIN_DEPS_WIT_FILE_NAME);

        if !std::fs::exists(&wit_path)? {
            // TODO: warn that the file does not exist
            return Ok(());
        }

        let mut resolve = Resolve::default();
        let package_id = resolve.push_file(&wit_path)?;

        let world_id = resolve.select_world(package_id, Some("deps"))?;

        match &self.language {
            BindingsLanguage::Rust => {
                // TODO: If wit-bindgen is not in Cargo.toml, make sure to add it.
                let opts = wit_bindgen_rust::Opts {
                    generate_all: true,
                    // TODO: Make the extra attributes a clap option
                    additional_derive_attributes: vec![
                        "serde::Serialize".to_string(),
                        "serde::Deserialize".to_string(),
                        "Hash".to_string(),
                        "Clone".to_string(),
                        "PartialEq".to_string(),
                        "Eq".to_string(),
                    ],
                    // Uncomment this once spin-sdk is updated and remove dependency on wit_bindgen in Cargo.toml
                    //runtime_path: Some("::spin_sdk::wit_bindgen".to_string()),
                    ..Default::default()
                };

                let mut generator = opts.build();

                let mut files = wit_bindgen_core::source::Files::default();
                generator.generate(&resolve, world_id, &mut files)?;

                fs::create_dir_all(&self.output).await?;

                let mut mod_output = String::new();

                for (name, contents) in files.iter() {
                    let output_path = self.output.join(name);
                    let mod_file = PathBuf::from(name);
                    let mod_name = mod_file.file_stem().unwrap().to_string_lossy();
                    std::fmt::write(&mut mod_output, format_args!("pub mod {mod_name};\n"))?;
                    fs::write(output_path, contents).await?;
                }

                fs::write(self.output.join("mod.rs"), mod_output).await?;
                println!("Bindings generated for Rust in {0}. You need to add the `wit-bindgen` crate to your Rust Spin app - e.g., `cargo add wit-bindgen`", self.output.to_str().expect("Failed to parse output path"));
            }
            BindingsLanguage::Ts => {
                todo!("generate ts")
            }
        }

        Ok(())
    }
}
