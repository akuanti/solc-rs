//! Call the Solidity compiler

use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::command::CompileCommand;

pub mod command;
pub mod compiler;
mod utils;

// TODO: return Result
fn load_bytes<P>(path: P) -> Vec<u8>
where
    P: AsRef<Path> + Debug,
{
    match File::open(&path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut contents: Vec<u8> = Vec::new();

            match reader.read_to_end(&mut contents) {
                Ok(_) => contents,
                Err(e) => panic!("Problem reading file {}", e),
            }
        }
        Err(e) => panic!("Could not open file {:?}: {}", path, e),
    }
}

#[cfg(test)]
mod test {
    use crate::compiler::Solc;
    use std::path::PathBuf;

    #[test]
    #[ignore]
    fn test_compile() {
        let compiler = Solc::new("test");
        compiler
            .command()
            .execute()
            .expect("No command")
            .output()
            .expect("Problem executing command");
    }

    #[test]
    fn should_convert_root_to_absolute_path() {
        let compiler = Solc::new("../test");
        let p = PathBuf::from(compiler.root());
        assert!(p.is_absolute(), "Root is not absolute path");
    }

    // check for solc exe
    // compile
    // load bytecode
    // load unlinked bytecode
}
