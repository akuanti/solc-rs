//! Commands for the compiler

use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct CompileSettings {
    pub root: PathBuf,
    pub allow_paths: Vec<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub libraries_file: Option<PathBuf>,
}

#[derive(Clone, Debug)]
/// Available outputs when outputting to separate files
pub enum SeparateOutput {
    Ast,
    AstJson,
    AstCompactJson,
    Asm,
    AsmJson,
    Opcodes,
    Bin,
    BinRuntime,
    Abi,
    Hashes,
    UserDoc,
    DevDoc,
    Metadata,
}

#[derive(Clone, Debug)]
/// Available outputs when outputting to combined JSON
pub enum CombinedOutput {
    Abi,
    Asm,
    Ast,
    Bin,
    BinRuntime,
    CompactFormat,
    DevDoc,
    Hashes,
    Interface,
    Metadata,
    Opcodes,
    SourceMap,
    SourceMapRuntime,
    UserDoc,
}

#[derive(Debug)]
/// Possible compiler outputs
pub enum CompileOutput {
    None,
    // TODO: these should be sets so there are no duplicates
    Separate(Vec<SeparateOutput>),
    CombinedJson(Vec<CombinedOutput>),
}

#[derive(Debug)]
/// Build up the compile command.
/// All paths are relative to the root
pub struct CompileCommand {
    root: PathBuf,
    allow_paths: Vec<PathBuf>,
    /// dll -> path
    mappings: HashMap<String, PathBuf>,
    // input
    source_files: Vec<PathBuf>,
    libraries: Option<PathBuf>,
    link: bool,
    // output types
    outputs: CompileOutput,
    abi: Option<()>,
    bin: Option<()>,
    // combined_json
    // output
    overwrite: bool,
    /// default: current directory
    output_dir: Option<PathBuf>,
    command: Option<Command>,
}

impl Default for CompileCommand {
    fn default() -> Self {
        CompileCommand {
            root: PathBuf::from("."),
            allow_paths: vec![],
            mappings: HashMap::new(),
            source_files: vec![],
            libraries: None,
            link: false,
            outputs: CompileOutput::None,
            abi: None,
            bin: None,
            overwrite: false,
            output_dir: Some(".".into()),
            command: None,
        }
    }
}

impl CompileCommand {
    /// Create a new `CompileCommand` with a given root
    pub fn new<P>(root: P) -> CompileCommand
    where
        P: AsRef<Path>,
    {
        let mut cmd = CompileCommand::default();
        cmd.root = PathBuf::from(root.as_ref());

        cmd
    }

    /// Create a new `CompileCommand` with the given settings
    pub fn from_settings(settings: CompileSettings) -> CompileCommand {
        let mut cmd = CompileCommand::default();
        cmd.root = settings.root;
        cmd.output_dir = settings.output_dir;
        cmd.allow_paths = settings.allow_paths;
        cmd.libraries = None;

        cmd
    }

    /// Authorize `solc` to search in the given path for includes
    pub fn allow_path<P>(&mut self, path: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.allow_paths.push(path.as_ref().to_owned());
        self
    }

    /// Output `.abi` files
    pub fn abi(&mut self) -> &mut Self {
        self.add_separate(&SeparateOutput::Abi)
    }

    /// Output `.bin` files (bytecode)
    pub fn bin(&mut self) -> &mut Self {
        self.add_separate(&SeparateOutput::Bin)
    }

    /// Output separate files
    pub fn outputs(&mut self, formats: &[SeparateOutput]) -> &mut Self {
        for fmt in formats {
            self.add_separate(fmt);
        }
        self
    }

    /// Add an output type to be created in the output directory
    fn add_separate(&mut self, output: &SeparateOutput) -> &mut Self {
        match self.outputs {
            CompileOutput::None => {
                self.outputs = CompileOutput::Separate(vec![output.clone()]);
            }
            CompileOutput::Separate(ref mut outputs) => {
                outputs.push(output.clone());
            }
            CompileOutput::CombinedJson(_) => panic!("Cannot combine combined and separate modes"),
        }
        self
    }

    /// Output combined JSON file
    pub fn combined_json(&mut self, formats: &[CombinedOutput]) -> &mut Self {
        for fmt in formats {
            self.add_combined(fmt);
        }
        self
    }

    /// Add an output type to be included in the combined JSON file
    fn add_combined(&mut self, output: &CombinedOutput) -> &mut Self {
        match self.outputs {
            CompileOutput::None => {
                self.outputs = CompileOutput::CombinedJson(vec![output.clone()]);
            }
            CompileOutput::CombinedJson(ref mut outputs) => {
                outputs.push(output.clone());
            }
            CompileOutput::Separate(_) => panic!("Cannot combine combined and separate modes"),
        }
        self
    }

    /// Add a source `.sol` file
    pub fn add_source<P>(&mut self, path: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.source_files.push(path.as_ref().to_owned());
        self
    }

    /// Add a mapping for includes
    pub fn add_mapping<P>(&mut self, lib: &str, path: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.mappings
            .insert(lib.to_owned(), path.as_ref().to_owned());
        self
    }

    /// Include libraries in compilation
    pub fn link(&mut self) -> &mut Self {
        self.link = true;
        self
    }

    /// Set the file in which to store the library addresses for linking
    fn libraries_file<P>(&mut self, path: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.libraries = Some(path.as_ref().to_owned());
        self
    }

    /// Overwrite existing outputs
    pub fn overwrite(&mut self) -> &mut Self {
        self.overwrite = true;
        self
    }

    /// Set the location of the build artifacts
    fn output_dir<P>(&mut self, path: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.output_dir = Some(path.as_ref().to_owned());
        self
    }

    // TODO: add EPM package remapping

    /// Get the command that will be executed in the shell
    pub fn command_line(&self) -> String {
        let line = format!("{:?}", self.command);
        line
    }

    /// Build up the shell command for compiling
    pub fn build(&mut self) {
        let mut cmd = Command::new("solc");

        cmd.current_dir(&self.root);

        // input config
        if !self.allow_paths.is_empty() {
            cmd.arg("--allow-paths");
            cmd.args(&self.allow_paths);
        }

        for (k, v) in &self.mappings {
            // remove the double quotes
            let p = v
                .to_str()
                .expect("Could not convert path to str")
                .trim_matches('"');
            let line = format!("{}={}", k, p);
            cmd.arg(line);
        }

        // output types
        // println!("OUTPUTS {:?}", self.outputs);
        match &self.outputs {
            CompileOutput::None => (),
            CompileOutput::Separate(outputs) => {
                let args: Vec<&str> = outputs
                    .iter()
                    .map(|output| match output {
                        SeparateOutput::Abi => "--abi",
                        SeparateOutput::Asm => "--asm",
                        SeparateOutput::AsmJson => "--asm-json",
                        SeparateOutput::Ast => "--ast",
                        SeparateOutput::AstJson => "--ast-json",
                        SeparateOutput::AstCompactJson => "--ast-compact-json",
                        SeparateOutput::Bin => "--bin",
                        SeparateOutput::BinRuntime => "--bin-runtime",
                        SeparateOutput::Hashes => "--hashes",
                        SeparateOutput::Metadata => "--metadata",
                        SeparateOutput::Opcodes => "--opcodes",
                        SeparateOutput::DevDoc => "--devdoc",
                        SeparateOutput::UserDoc => "--userdoc",
                    })
                    .collect();
                cmd.args(args);
            }
            CompileOutput::CombinedJson(outputs) => {
                let args: Vec<&str> = outputs
                    .iter()
                    .map(|output| match output {
                        CombinedOutput::Abi => "abi",
                        CombinedOutput::Asm => "asm",
                        CombinedOutput::Ast => "ast",
                        CombinedOutput::Bin => "bin",
                        CombinedOutput::BinRuntime => "bin-runtime",
                        CombinedOutput::CompactFormat => "compact-format",
                        CombinedOutput::DevDoc => "devdoc",
                        CombinedOutput::Hashes => "hashes",
                        CombinedOutput::Interface => "interface",
                        CombinedOutput::Metadata => "metadata",
                        CombinedOutput::Opcodes => "opcodes",
                        CombinedOutput::SourceMap => "srcmap",
                        CombinedOutput::SourceMapRuntime => "srcmap-runtime",
                        CombinedOutput::UserDoc => "userdoc",
                    })
                    .collect();
                cmd.args(&["--combined-json", &args.join(",")]);
            }
        }

        // If `libraries` is set, add it to the command
        // currently only handles a path to a library file
        if self.link {
            // println!("adding link argument");
            if self.libraries.is_some() {
                // NOTE: if output path is None, this fails
                // this should not be the case
                match self.join_output_path("libs.txt") {
                    Ok(ref libraries_file) => {
                        cmd.arg("--libraries");
                        cmd.arg(libraries_file);
                    }
                    Err(e) => println!("Problem adding link argument {:?}", e),
                }
            }
        } else {
            // println!("not linking");
        }

        if self.overwrite {
            cmd.arg("--overwrite");
        }

        if let Some(ref dir) = self.output_dir {
            cmd.arg("-o");
            cmd.arg(dir.as_os_str());
        }

        // sources
        cmd.args(&self.source_files);

        println!("COMMAND: {:?}", cmd);
        self.command = Some(cmd);
    }

    // TODO: create a CompileError
    /// Execute the compile command in the shell
    pub fn execute(&mut self) -> Option<&mut Command> {
        if self.command.is_none() {
            self.build();
        }

        self.command.as_mut()
    }

    /// Add the given path to the output dir
    fn join_output_path<P>(&self, path: P) -> Result<PathBuf, &'static str>
    where
        P: AsRef<Path> + Debug,
    {
        match self.output_dir {
            Some(ref dir) => {
                let mut buf = PathBuf::new();
                buf.push(dir);
                buf.push(path);
                Ok(buf)
            }
            None => Err("Could not join path - output dir is not set"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn make_builder() {
        CompileCommand::new("test");
    }

    // test that each of the functions correctly add to the command line
    #[test]
    fn test_abi() {
        let mut builder = CompileCommand::new("test");
        builder.abi().build();
        let line = builder.command_line();
        assert!(line.as_str().contains("--abi"));
    }

    #[test]
    fn test_bin() {
        let mut builder = CompileCommand::new("test");
        builder.bin().build();
        assert!(builder.command_line().as_str().contains("--bin"));
    }

    #[test]
    fn test_allow_paths() {
        let mut builder = CompileCommand::new("test");
        builder.allow_path("somewhere").build();

        assert!(builder.command_line().as_str().contains("--allow-paths"));
        assert!(builder.command_line().as_str().contains("somewhere"));
    }

    #[test]
    fn test_add_source() {
        let mut builder = CompileCommand::new("test");
        builder.add_source("contracts/Test.sol").build();
        assert!(builder
            .command_line()
            .as_str()
            .contains("contracts/Test.sol"));
    }

    #[test]
    fn test_add_lib_file() {
        let mut builder = CompileCommand::new("test");
        builder.libraries_file("libs.txt").bin().link().build();
        println!("{:?}", builder);
        assert!(builder.command_line().as_str().contains("libs.txt"));
    }

    #[test]
    fn test_output_dir() {
        let mut builder = CompileCommand::new("test");
        builder.output_dir("output").build();

        assert!(builder.command_line().as_str().contains("-o"));
        assert!(builder.command_line().as_str().contains("output"));
    }

    #[test]
    fn test_overwrite() {
        let mut builder = CompileCommand::new("test");
        builder.overwrite().build();
        assert!(builder.command_line().as_str().contains("--overwrite"));
    }

    #[test]
    fn test_mapping() {
        let mut builder = CompileCommand::new("test");
        builder.add_mapping("lib", "path/to/lib").build();
        println!("{:}", builder.command_line().as_str());

        assert!(builder.command_line().as_str().contains("lib=path/to/lib"));
    }

    // libraries_file
    #[test]
    fn should_not_add_libs_if_not_in_command() {
        let mut builder = CompileCommand::new("test");
        builder.build();
        assert!(!builder.command_line().as_str().contains("--libraries"));
    }
    // output_dir

    fn strip_quotes(command: &str) -> String {
        command.replace("\"", "")
    }

    #[test]
    fn test_combined_json() {
        let mut builder = CompileCommand::new("test");
        builder
            .combined_json(&vec![CombinedOutput::Abi, CombinedOutput::Bin])
            .build();

        let line = strip_quotes(&builder.command_line());
        assert!(line.contains("--combined-json abi,bin"));
    }

    #[test]
    fn test_all_combined_json() {
        use CombinedOutput::*;
        let mut builder = CompileCommand::new("test");
        let all = vec![
            Abi,
            Asm,
            Ast,
            Bin,
            BinRuntime,
            CompactFormat,
            DevDoc,
            Hashes,
            Interface,
            Metadata,
            Opcodes,
            SourceMap,
            SourceMapRuntime,
            UserDoc,
        ];
        builder.combined_json(&all).build();

        let line = strip_quotes(&builder.command_line());
        println!("{:?}", line);
        assert!(line.contains(
            "--combined-json abi,asm,ast,bin,bin-runtime,compact-format,devdoc,hashes,interface,metadata,opcodes,srcmap,srcmap-runtime,userdoc"
        ));
    }

    #[test]
    fn test_multiple_outputs() {
        let mut builder = CompileCommand::new("test");
        builder
            .outputs(&vec![SeparateOutput::Abi, SeparateOutput::Bin])
            .build();

        let line = builder.command_line();
        assert!(line.contains("--abi"));
        assert!(line.contains("--bin"));
    }

    #[test]
    fn test_all_outputs() {
        use SeparateOutput::*;
        let mut builder = CompileCommand::new("test");
        let all = vec![
            Ast,
            AstJson,
            AstCompactJson,
            Asm,
            AsmJson,
            Opcodes,
            Bin,
            BinRuntime,
            Abi,
            Hashes,
            UserDoc,
            DevDoc,
            Metadata,
        ];
        builder.outputs(&all).build();

        let line = builder.command_line();
        assert!(line.contains("--ast"));
        assert!(line.contains("--ast-json"));
        assert!(line.contains("--ast-compact-json"));
        assert!(line.contains("--asm"));
        assert!(line.contains("--asm-json"));
        assert!(line.contains("--opcodes"));
        assert!(line.contains("--bin"));
        assert!(line.contains("--bin-runtime"));
        assert!(line.contains("--abi"));
        assert!(line.contains("--hashes"));
        assert!(line.contains("--userdoc"));
        assert!(line.contains("--devdoc"));
        assert!(line.contains("--metadata"));
    }

    // test join_output_dir
    // test join_root
}
