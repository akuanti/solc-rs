//! Call the Solidity compiler
extern crate rustc_hex;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::collections::HashMap;

use rustc_hex::FromHex;

/// Build up the compile command
#[derive(Debug)]
pub struct CompileCommand<'a> {
    root: String,
    allow_paths: Vec<String>,
    mappings: HashMap<String, String>,
    // input
    source_files: Vec<String>,
    libraries: Option<&'a str>,
    // output types
    abi: Option<()>,
    bin: Option<()>,
    // combined_json
    // output
    overwrite: bool,
    output_dir: Option<&'a str>,
    command: Option<Command>,
}

impl<'a> CompileCommand<'a> {
    pub fn new(root: &str) -> CompileCommand {
        CompileCommand {
            root: root.to_owned(),
            allow_paths: Vec::new(),
            mappings: HashMap::new(),
            source_files: Vec::new(),
            libraries: None,
            abi: None,
            bin: None,
            overwrite: false,
            output_dir: None,
            command: None,
        }
    }

    pub fn allow_path(&mut self, path: &str) -> &mut Self {
        self.allow_paths.push(path.to_owned());
        self
    }

    pub fn abi(&mut self) -> &mut Self {
        self.abi = Some(());
        self
    }

    pub fn bin(&mut self) -> &mut Self {
        self.bin = Some(());
        self
    }

    pub fn add_source(&mut self, path: &str) -> &mut Self {
        self.source_files.push(path.to_owned());
        self
    }

    pub fn add_mapping(&mut self, lib: &str, path: &str) -> &mut Self {
        self.mappings.insert(lib.to_owned(), path.to_owned());
        self
    }

    pub fn libraries_file(&mut self, path: &'a str) -> &mut Self {
        self.libraries = Some(path);
        self
    }

    pub fn overwrite(&mut self) -> &mut Self {
        self.overwrite = true;
        self
    }

    pub fn output_dir(&mut self, path: &'a str) -> &mut Self {
        self.output_dir = Some(path);
        self
    }

    // TODO: add EPM package remapping

    pub fn command_line(&self) -> String {
        let line = format!("{:?}", self.command);
        line
    }

    pub fn go(&mut self) {
        let mut cmd = Command::new("solc");

        cmd.current_dir(&self.root);

        // input config
        if self.allow_paths.len() > 0 {
            cmd.arg("--allow-paths");
            cmd.args(&self.allow_paths);
        }

        for (k, v) in self.mappings.iter() {
            let line = format!("{}={}", k, v);
            cmd.arg(line);
        }

        // output types
        if let Some(_) = self.abi {
            cmd.arg("--abi");
        }

        if let Some(_) = self.bin {
            cmd.arg("--bin");
        }

        if let Some(f) = self.libraries {
            cmd.args(&["--libraries", f]);
        }

        if self.overwrite {
            cmd.arg("--overwrite");
        }

        if let Some(dir) = self.output_dir {
            cmd.args(&["-o", dir]);
        }

        // sources
        cmd.args(&self.source_files);

        // println!("COMMAND: {:?}", cmd);
        self.command = Some(cmd);
    }

    // TODO: create a CompileError
    pub fn execute(&mut self) -> Option<&mut Command> {
        if let None = self.command {
            self.go();
        }

        self.command.as_mut()
    }
}

#[derive(Debug)]
pub struct Solc<'a> {
    root: String,
    pub output_dir: Option<&'a str>,
    allow_paths: Vec<String>,
}

impl<'a> Solc<'a> {
    pub fn new(root: &str) -> Self {
        Solc {
            root: root.to_owned(),
            output_dir: None,
            allow_paths: Vec::<String>::new(),
        }
    }

    pub fn root(&self) -> &str {
        &self.root[..]
    }

    pub fn output_dir(&self) -> &str {
        self.output_dir.unwrap()
    }

    // load from <root>/<output_dir>/<name>
    // only load LINKED bytecode
    pub fn load_bytecode(&self, name: &str) -> Vec<u8> {
        match self.output_dir {
            Some(ref dir) => {
                let bytecode_path: PathBuf = [self.root.as_str(), dir, name].iter().collect();
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

    // load from <root>/<output_dir>/<name>
    pub fn load_abi(&self, name: &str) -> Vec<u8> {
        match self.output_dir {
            Some(ref dir) => {
                let abi_path: PathBuf = [self.root.as_str(), dir, name].iter().collect();
                let path: &str = abi_path.to_str().unwrap();
                load_bytes(path)
            }
            None => panic!("No output path set"),
        }
    }

    pub fn compile(&self) -> CompileCommand {
        // TODO: add allow_paths here
        CompileCommand::new(self.root())
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn test_compile() {
        let compiler = Solc::new("test");
        compiler
            .compile()
            .execute()
            .expect("No command")
            .output()
            .expect("Problem executing command");
    }

    #[test]
    fn make_builder() {
        CompileCommand::new("test");
    }

    // test that each of the functions correctly add to the command line
    #[test]
    fn test_abi() {
        let mut builder = CompileCommand::new("test");
        builder.abi().go();
        let line = builder.command_line();
        assert!(line.as_str().contains("--abi"));
    }

    #[test]
    fn test_bin() {
        let mut builder = CompileCommand::new("test");
        builder.bin().go();
        assert!(builder.command_line().as_str().contains("--bin"));
    }

    #[test]
    fn test_allow_paths() {
        let mut builder = CompileCommand::new("test");
        builder.allow_path("somewhere").go();

        assert!(builder.command_line().as_str().contains("--allow-paths"));
        assert!(builder.command_line().as_str().contains("somewhere"));
    }

    #[test]
    fn test_add_source() {
        let mut builder = CompileCommand::new("test");
        builder.add_source("contracts/Test.sol").go();
        assert!(
            builder
                .command_line()
                .as_str()
                .contains("contracts/Test.sol")
        );
    }

    #[test]
    fn test_add_lib_file() {
        let mut builder = CompileCommand::new("test");
        builder.libraries_file("libs.txt").go();
        assert!(builder.command_line().as_str().contains("libs.txt"));
    }

    #[test]
    fn test_output_dir() {
        let mut builder = CompileCommand::new("test");
        builder.output_dir("output").go();

        assert!(builder.command_line().as_str().contains("-o"));
        assert!(builder.command_line().as_str().contains("output"));
    }

    #[test]
    fn test_overwrite() {
        let mut builder = CompileCommand::new("test");
        builder.overwrite().go();
        assert!(builder.command_line().as_str().contains("--overwrite"));
    }

    #[test]
    fn test_mapping() {
        let mut builder = CompileCommand::new("test");
        builder.add_mapping("lib", "path/to/lib").go();

        assert!(builder.command_line().as_str().contains("lib=path/to/lib"));
    }

    // check for solc exe
    // compile
    // load bytecode
    // load unlinked bytecode
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
