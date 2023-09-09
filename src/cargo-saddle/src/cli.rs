use std::fs;

use anyhow::Context;
use clap::{Parser, Subcommand};

use crate::{
    decoder::{decode_binary, DecoderEntryKind},
    validator::{
        BorrowMeta, CallMeta, ComponentId, ComponentMeta, Mutability, ScopeId, ScopeMeta, Validator,
    },
};

#[derive(Debug, Parser)]
#[command(
	author = "radbuglet",
	version = "0.1.0",
	about = "Statically validates saddle borrow rules on the chosen binary",
	long_about = None,
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command()]
    Check {
        #[arg(help = "The path to the binary being analyzed", long = None)]
        path: String,
    },
}

pub fn main_inner() -> anyhow::Result<()> {
    color_backtrace::install();
    let args = Args::parse();

    match &args.command {
        Commands::Check { path } => {
            // Load the file
            let file = fs::read(path).context("failed to read file to be analyzed")?;

            // Load all declarations
            let mut validator = Validator::default();
            let mut found_any_directive = false;

            decode_binary(&file, |mode, arg_1, arg_2| {
                found_any_directive = true;

                validator.annotate_scope(
                    ScopeId(arg_1.clone(), []),
                    ScopeMeta {
                        _dummy: [],
                        name: arg_1.clone(),
                        defined_at: "<unknown>",
                    },
                );

                match mode {
                    DecoderEntryKind::DepRef | DecoderEntryKind::DepMut => {
                        let mutability = if mode == DecoderEntryKind::DepRef {
                            Mutability::Immutable
                        } else {
                            Mutability::Mutable
                        };

                        validator.annotate_component(
                            ComponentId(arg_2.clone(), []),
                            ComponentMeta {
                                _dummy: [],
                                name: arg_2.clone(),
                            },
                        );

                        validator.push_access(
                            ScopeId(arg_1, []),
                            ComponentId(arg_2, []),
                            mutability,
                            BorrowMeta {
                                def_path: "<unknown>",
                                mutability,
                            },
                        );
                    }
                    DecoderEntryKind::Calls => validator.push_call_edge(
                        ScopeId(arg_1, []),
                        ScopeId(arg_2, []),
                        CallMeta {
                            def_path: "<unknown>",
                        },
                    ),
                }
            })?;

            anyhow::ensure!(
                found_any_directive,
                "Did not find any saddle directives while scanning binary."
            );

            // Validate graph
            validator.validate()?;

            eprintln!("Binary is valid.");
            Ok(())
        }
    }
}
