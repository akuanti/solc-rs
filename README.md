# Solc

A Rust interface to the Solidity compiler.

## Introduction
The basic usage of the Solidity compiler (`solc`) is that you specify the input files to be compiled and the types of outputs you would like. In addition, you may specify *how* you want to compile (i.e. optimizations, search paths, etc.).

This library follows a similar model, but it abstracts away some of the complicated parts with a struct `Solc`, then uses a builder to build up the command, which you can then execute.

The steps are:
- create a compiler object
- generate a builder
- add inputs, options, and outputs to the builder
- execute the shell command

## Basic compilation
At minimum, you need to create the compiler and set its working and output directories, then add a source file to compile.

```rust
// instantiate a compiler with a compilation directory relative to the current directory
let mut compiler = solc::Solc::new(".");
let build_dir = "build";
compiler.output_dir = Some(build_dir);

// make a builder that will output a .bin and a .abi and add the desired source to it
let mut builder = compiler.compile();
builder
    .bin()
    .abi()
    .add_source("contracts/MyContract.sol");

// execute the compilation
let cmd = builder.execute().expect("No command");
let _output = cmd.output().expect("Failed to compile contract"); 
```

# Linking
If you are compiling a contract that depends on a library, the compiler will leave placeholders in the compiled bytecode by default. When you **link** the contract with the library, the compiler will replace those placeholders with the address of the deployed library.

The `Solc` struct lets you store the name-address mappings of libraries it needs access to, so that when you compile, it can link at the same time. You need to call `prepare_link()` to write out the library information to a file for inclusion in later compilations. When you build up the compile command, you can then add the `link()` option so that these mappings can be read in from the file and provided to the shell command.

```rust
// add a library to be linked
compiler.add_library_address("DLL", dll.address());
// save the mappings for later use
compiler.prepare_link();

// compile a contract, including the previously added libraries for linking
let mut builder = compiler.compile();
builder
    .bin()
    .link()
    .add_source("contracts/MyContract.sol");

// execute the compilation
let cmd = builder.execute().expect("No command");
let _output = cmd.output().expect("Failed to compile contract");
```


# Including contracts from EPM packages
EPM puts its packages in `installed_contracts`. For example, if you install `dll`, a doubly-linked list, the contracts will be in `installed_contracts/dll/contracts/`. Truffle allows you to reference `DLL.sol` in this directory by using `import dll/DLL.sol`. In order to make this work when calling `solc` directly, you need to introduce mappings. In this case, the mapping would say, when you see `dll/` in an import statement, treat it as `installed_contracts/dll/contracts/`.

However, `solc` restricts where you can import from for safety, so you need also to explicitly tell it that `installed_contracts` is a safe place to install contracts from.


```rust
builder
    .add_mapping("dll", "installed_contracts/dll/contracts")
    .allow_path("/abs/path/to/installed_contracts");
```
