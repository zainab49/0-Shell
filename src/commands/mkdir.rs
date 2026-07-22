//! `mkdir` — create directories.

use std::fs;

use crate::commands::CmdResult;

/// Creates each directory operand. Supports `-p`, which creates parent
/// directories as needed and treats an existing target as success.
pub fn run(args: &[String]) -> CmdResult {
    let (parents, operands) = match args.first() {
        Some(flag) if flag == "-p" => (true, &args[1..]),
        _ => (false, args),
    };

    if operands.is_empty() {
        return Err("missing operand".to_string());
    }

    for dir in operands {
        let result = if parents {
            fs::create_dir_all(dir)
        } else {
            fs::create_dir(dir)
        };
        if let Err(e) = result {
            eprintln!("mkdir: cannot create directory '{dir}': {e}");
        }
    }

    Ok(())
}
