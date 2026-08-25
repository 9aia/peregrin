use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "handl", about = "handl language — evaluate, compile, typecheck")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Evaluate an expression and output the result.
    Eval(EvalArgs),
    /// Compile an handl expression to another target.
    Compile(CompileArgs),
    /// Static type checking of `$:` declarations.
    Typecheck(TypecheckArgs),
}

#[derive(Args)]
pub struct EvalArgs {
    /// Input files. Reads from stdin if none are given.
    #[arg(value_name = "INPUT")]
    pub input: Vec<std::path::PathBuf>,

    /// Input format (handl or toml). Defaults to handl.
    #[arg(short = 'f', long, default_value = "handl")]
    pub format: String,

    /// Output format (toml). Defaults to toml.
    #[arg(short = 'o', long, default_value = "toml")]
    pub output: String,

    /// VM host to use: sandbox (default, pure) or server (effects enabled).
    #[arg(short = 'v', long, default_value = "sandbox")]
    pub vm: String,
}

#[derive(Args)]
pub struct CompileArgs {
    /// Input file. Reads from stdin if none are given.
    #[arg(value_name = "INPUT")]
    pub input: Vec<std::path::PathBuf>,

    /// Input format (handl or toml). Defaults to handl.
    #[arg(short = 'f', long, default_value = "handl")]
    pub format: String,

    /// Output file path. Writes to stdout if omitted.
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Target language (js).
    #[arg(short = 't', long)]
    pub target: String,
}

#[derive(Args)]
pub struct TypecheckArgs {
    /// Input files. Reads from stdin if none are given.
    #[arg(value_name = "INPUT")]
    pub input: Vec<std::path::PathBuf>,

    /// Input format (handl or toml). Defaults to handl.
    #[arg(short = 'f', long, default_value = "handl")]
    pub format: String,
}
