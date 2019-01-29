//! Solidity compiler

use ethereum_types::Address;
use rustc_hex::FromHex;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::str;

use crate::command::CompileSettings;
use crate::{utils, CompileCommand};

#[derive(Debug)]
struct LibraryMapping {
    name: String,
    address: Address,
}

#[derive(Debug)]
/// Wrapper around the Solidity compiler
/// When you call compile(), you get a `CompileCommand`. Calling `execute()` on the `CompileCommand`
/// moves to the root directory and actually calls `solc`
pub struct Solc<'a> {
    /// Absolute path to compiler root
    root: PathBuf,
    pub output_dir: Option<&'a str>,
    allow_paths: Vec<String>,
    /// relative to output dir
    lib_file: &'a str,
    /// library mappings for linking
    libraries: Vec<LibraryMapping>,
    // TODO: add exe-path
}

impl<'a> Solc<'a> {
    /// Creates a new `Solc` that operates in the `root` directory
    pub fn new<P>(root: P) -> Self
    where
        P: AsRef<Path>,
    {
        // convert root to absolute path
        // TODO: fail if this does not work?
        let mut p = PathBuf::new();
        p.push(root);
        let root_abs = utils::absolute(p.as_path());

        Solc {
            root: root_abs,
            output_dir: None,
            allow_paths: Vec::<String>::new(),
            lib_file: "libs.txt",
            libraries: Vec::new(),
        }
    }

    /// Returns the directory compiler's working directory
    pub fn root(&self) -> &str {
        self.root.as_os_str().to_str().expect("Could not get root")
    }

    /// Returns the directory where the compiler output files go
    pub fn output_dir(&self) -> &str {
        self.output_dir.unwrap_or("")
    }

    /// Add library address for linking
    pub fn add_library_address(&mut self, name: &str, address: Address) {
        self.libraries.push(LibraryMapping {
            name: name.to_string(),
            address,
        });
    }

    /// Write out the library file from the libraries
    // TODO: don't actually save to a file?
    pub fn prepare_link(&self) {
        if let Some(dir) = self.output_dir {
            match utils::join_path(dir, self.lib_file) {
                Ok(ref path) => {
                    // want <root>/<path>
                    let mut full_path = PathBuf::from(self.root());
                    full_path.push(path);
                    let mut lib_file = File::create(full_path).expect("Could not create libs file");

                    // write each library to the file
                    for lib in &self.libraries {
                        if let Err(e) = writeln!(lib_file, "{}:{:?}", lib.name, lib.address) {
                            eprintln!("Couldn't write to library file: {}", e);
                        }
                    }
                }
                // TODO: deal with this properly
                Err(_) => panic!("Problem with lib file path"),
            } // end join_path
        } // end self.output_dir
    }

    // load from <root>/<output_dir>/<name>
    // only loads LINKED bytecode
    // TODO: return Result
    pub fn load_bytecode(&self, name: &str) -> Vec<u8> {
        match self.output_dir {
            Some(ref dir) => {
                let bytecode_path: PathBuf = [self.root(), dir, name].iter().collect();
                println!("bytecode at: {:?}", bytecode_path);
                // TODO: use combinators
                let path = format!("{}", bytecode_path.display());
                let bytes = load_bytes(&path[..]);
                let code = str::from_utf8(&bytes[..]).unwrap();
                // println!("CODE: {}", code);
                // bytecode_path.as_path()
                code.from_hex().unwrap()
                // code
            }
            None => panic!("No output path set"),
        }
    }

    /// Load a given ABI file from the output directory
    /// name is the file name
    pub fn load_abi(&self, name: &str) -> Vec<u8> {
        match self.output_dir {
            Some(ref dir) => {
                let abi_path: PathBuf = [self.root(), dir, name].iter().collect();
                let path: &str = abi_path.to_str().unwrap();
                load_bytes(path)
            }
            None => panic!("No output path set"),
        }
    }

    /// Generate a `CompileCommand` from the compiler for building
    /// up the compilation.
    pub fn command(&self) -> CompileCommand {
        // TODO: add allow_paths here
        let settings = CompileSettings {
            root: PathBuf::from(self.root()),
            allow_paths: vec![],
            output_dir: Some(PathBuf::from(self.output_dir())),
            libraries_file: Some(PathBuf::from(self.lib_file)),
        };
        CompileCommand::from_settings(settings)
    }
}

// TODO: return Result
fn load_bytes(path: &str) -> Vec<u8> {
    match File::open(path) {
        Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut contents: Vec<u8> = Vec::new();

            match reader.read_to_end(&mut contents) {
                Ok(_) => contents,
                Err(e) => panic!("Problem reading file {}", e),
            }
        }
        Err(e) => panic!("Could not open file {}: {}", path, e),
    }
}
