//! Call the Solidity compiler
extern crate ethereum_types;
extern crate rustc_hex;

use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use std::collections::HashMap;

use ethereum_types::Address;

use rustc_hex::FromHex;

#[derive(Debug)]
/// Build up the compile command.
/// All paths are relative to the root
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
    /// Create a new `CompileCommand` with a given root
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

    /// Authorize `solc` to search in the given path for includes
    pub fn allow_path(&mut self, path: &str) -> &mut Self {
        self.allow_paths.push(path.to_owned());
        self
    }

    /// Output `.abi` files
    pub fn abi(&mut self) -> &mut Self {
        self.abi = Some(());
        self
    }

    /// Output `.bin` files (bytecode)
    pub fn bin(&mut self) -> &mut Self {
        self.bin = Some(());
        self
    }

    /// Add a source `.sol` file
    pub fn add_source(&mut self, path: &str) -> &mut Self {
        self.source_files.push(path.to_owned());
        self
    }

    /// Add a mapping for includes
    pub fn add_mapping(&mut self, lib: &str, path: &str) -> &mut Self {
        self.mappings.insert(lib.to_owned(), path.to_owned());
        self
    }

    // TODO: make this link()?
    /// Set the file in which to store the library addresses for linking
    fn libraries_file(&mut self, path: &'a str) -> &mut Self {
        self.libraries = Some(path);
        self
    }

    /// Overwrite existing outputs
    pub fn overwrite(&mut self) -> &mut Self {
        self.overwrite = true;
        self
    }

    /// Set the location of the build artifacts
    fn output_dir(&mut self, path: &'a str) -> &mut Self {
        self.output_dir = Some(path);
        self
    }

    // TODO: add EPM package remapping

    /// Get the command that will be executed in the shell
    pub fn command_line(&self) -> String {
        let line = format!("{:?}", self.command);
        line
    }

    /// Build up the shell command for compiling
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

        // If `libraries` is set, add it to the command
        // currently only handles a path to a library file
        if let Some(_) = self.libraries {
            match self.join_output_path("libs.txt") {
                Ok(ref libraries_file) => {
                    cmd.arg("--libraries");
                    cmd.arg(libraries_file);
                }
                Err(_) => (),
            }
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
    /// Execute the compile command in the shell
    pub fn execute(&mut self) -> Option<&mut Command> {
        if let None = self.command {
            self.go();
        }

        self.command.as_mut()
    }

    /// Add the given path to the output dir
    fn join_output_path<P>(&self, path: P) -> Result<PathBuf, &'static str>
    where
        P: AsRef<Path> + Debug,
    {
        match self.output_dir {
            Some(dir) => {
                let mut buf = PathBuf::from(dir);
                buf.push(path);
                Ok(buf)
            }
            None => Err("Could not join path to the output dir"),
        }
    }
}

/// Join the path
fn join_path<P>(base: &str, path: P) -> Result<PathBuf, &'static str>
where
    P: AsRef<Path> + Debug,
{
    let mut buf = PathBuf::from(base);
    buf.push(path);
    Ok(buf)
}

/// Get the absolute path of a given path
fn absolute(path: &Path) -> PathBuf {
    // println!("path: {:?}", path);
    let result = path.to_path_buf();
    let mut absolute_path = PathBuf::new();
    if !result.is_absolute() {
        match env::current_dir() {
            Ok(current_dir) => absolute_path.push(current_dir),
            Err(_) => println!("Could not get current directory"),
        }
    }

    absolute_path.push(result);

    // println!("abs_path: {:?}", absolute_path.as_path());
    absolute_path
}

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
}

impl<'a> Solc<'a> {
    /// Creates a new `Solc` that operates in the `root` directory
    pub fn new(root: &str) -> Self {
        // convert root to absolute path
        let mut p = PathBuf::new();
        p.push(root);
        let root_abs = absolute(p.as_path());
        let root_abs = root_abs
            .canonicalize()
            .expect("Could not calculate compiler root");

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
        match self.output_dir {
            Some(dir) => {
                match join_path(dir, self.lib_file) {
                    Ok(ref path) => {
                        // want <root>/<path>
                        let mut full_path = PathBuf::from(self.root());
                        full_path.push(path);
                        let mut lib_file =
                            File::create(full_path).expect("Could not create libs file");

                        // write each library to the file
                        for lib in self.libraries.iter() {
                            if let Err(e) = writeln!(lib_file, "{}:{:?}", lib.name, lib.address) {
                                eprintln!("Couldn't write to library file: {}", e);
                            }
                        }
                    }
                    // TODO: deal with this properly
                    Err(_) => panic!("Problem with lib file path"),
                } // end join_path
            }
            None => (),
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

    pub fn compile(&self) -> CompileCommand {
        // TODO: add allow_paths here
        let mut cmd = CompileCommand::new(self.root());
        // set the output dir of the compiler, relative to its root
        // output_dir = "../tcr/output"
        // output_dir_absolute = "/path/to/tcr/output"
        // root = "/path/to/tcr"
        let output_dir_relative = self.output_dir();
        println!("OUTPUT: {}", output_dir_relative);
        cmd.output_dir(output_dir_relative);

        cmd.libraries_file(self.lib_file);
        cmd
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
    fn absolute_should_convert_relative_path() {
        let p = PathBuf::from("../test");
        let a = absolute(p.as_path());
        assert!(a.is_absolute(), "Path is not absolute");
        assert!(a.ends_with("test"));
    }

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
    fn should_convert_root_to_absolute_path() {
        let compiler = Solc::new("../test");
        let p = PathBuf::from(compiler.root());
        assert!(p.is_absolute(), "Root is not absolute path");
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
