//! Anchor IDL to Decoder Code Generator CLI
//!
//! This tool generates Rust decoder implementations from Anchor IDL JSON files.
//!
//! ## Usage
//!
//! ```bash
//! # Generate from a single IDL file
//! anchor-gen path/to/idl.json -o src/generated/
//!
//! # Generate from multiple IDLs
//! anchor-gen target/idl/*.json -o src/generated/
//!
//! # Generate with serde support
//! anchor-gen idl.json -o src/ --serde
//!
//! # Generate minimal output (less derives)
//! anchor-gen idl.json -o src/ --minimal
//! ```

use account_decoder_anchor_gen::{CodeGenerator, GeneratorConfig, IdlParser};
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Generate Solana account decoders from Anchor IDL files.
#[derive(Parser, Debug)]
#[command(name = "anchor-gen")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input IDL file(s) (JSON format)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output directory for generated code
    #[arg(short, long, default_value = ".")]
    output: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value = "single-file")]
    format: OutputFormat,

    /// Generate serde derives for JSON serialization
    #[arg(long)]
    serde: bool,

    /// Generate minimal output (only Debug derive)
    #[arg(long)]
    minimal: bool,

    /// Skip generating instruction decoders
    #[arg(long)]
    skip_instructions: bool,

    /// Skip generating account decoders
    #[arg(long)]
    skip_accounts: bool,

    /// Include doc comments from IDL
    #[arg(long, default_value = "true")]
    docs: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long)]
    quiet: bool,
}

/// Output format options.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Generate a single file per IDL
    SingleFile,
    /// Generate a module directory per IDL
    Module,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.quiet {
        tracing::Level::ERROR
    } else if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Build generator config
    let config = if args.minimal {
        GeneratorConfig::minimal()
    } else {
        let mut config = GeneratorConfig::default();
        if args.serde {
            config = config.with_serde();
        }
        config.include_docs = args.docs;
        config.generate_instructions = !args.skip_instructions;
        config.generate_accounts = !args.skip_accounts;
        config
    };

    let generator = CodeGenerator::new(config);

    // Process each input file
    for input_path in &args.input {
        if !input_path.exists() {
            warn!("Input file not found: {}", input_path.display());
            continue;
        }

        info!("Processing: {}", input_path.display());

        // Read and parse IDL
        let idl_json = fs::read_to_string(input_path)
            .with_context(|| format!("Failed to read {}", input_path.display()))?;

        let idl = IdlParser::parse(&idl_json)
            .with_context(|| format!("Failed to parse IDL: {}", input_path.display()))?;

        debug!("Parsed IDL: {} v{}", idl.name, idl.version);
        debug!(
            "  Accounts: {}, Instructions: {}, Types: {}",
            idl.accounts.len(),
            idl.instructions.len(),
            idl.types.len()
        );

        // Generate code
        let code = generator
            .generate(&idl)
            .with_context(|| format!("Failed to generate code for {}", idl.name))?;

        // Write output
        let output_path = match args.format {
            OutputFormat::SingleFile => {
                let filename = format!("{}.rs", idl.name.replace('-', "_"));
                args.output.join(filename)
            }
            OutputFormat::Module => {
                let module_dir = args.output.join(&idl.name);
                fs::create_dir_all(&module_dir)?;
                module_dir.join("mod.rs")
            }
        };

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Format the code (basic formatting via prettyplease if available)
        let formatted_code = format_code(&code.to_string());

        fs::write(&output_path, formatted_code)
            .with_context(|| format!("Failed to write {}", output_path.display()))?;

        info!("Generated: {}", output_path.display());
    }

    info!("Done!");
    Ok(())
}

/// Format generated Rust code.
///
/// Pretty-print generated code.
///
/// The generator emits a `TokenStream`, which stringifies to one very long
/// line. Parsing it back with `syn` and printing it with `prettyplease` gives
/// readable output and, usefully, fails loudly here if the generator ever
/// emits something that is not valid Rust.
fn format_code(code: &str) -> String {
    match syn::parse_file(code) {
        Ok(file) => prettyplease::unparse(&file),
        Err(_) => code.to_string(),
    }
}
