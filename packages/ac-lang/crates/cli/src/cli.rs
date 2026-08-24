use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ac",
    about = "AC language — parse, evaluate, validate, typecheck"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Parse a file and output the parsed representation.
    Parse(ParseArgs),
    /// Evaluate an expression and output the result.
    Eval(EvalArgs),
    /// Compile an AC expression to another target.
    Compile(CompileArgs),
    /// Validate a file against a schema file.
    Validate(ValidateArgs),
    /// Validate a self-describing file with inline `$:` schemas.
    Fields(FieldsArgs),
    /// Generate type definitions from `$:` declarations.
    Typegen(TypegenArgs),
    /// Static type checking of `$:` declarations.
    Typecheck(TypecheckArgs),
}

#[derive(Args)]
pub struct ParseArgs {
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    pub input: Option<std::path::PathBuf>,

    /// Input format (ac or toml). Defaults to toml.
    #[arg(short = 'f', long, default_value = "toml")]
    pub format: String,
}

#[derive(Args)]
pub struct EvalArgs {
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    pub input: Option<std::path::PathBuf>,

    /// Input format (ac or toml). Defaults to toml.
    #[arg(short = 'f', long, default_value = "toml")]
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
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    pub input: Option<std::path::PathBuf>,

    /// Input format (ac or toml). Defaults to ac.
    #[arg(short = 'f', long, default_value = "ac")]
    pub format: String,

    /// Output file path. Writes to stdout if omitted.
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Target language (js).
    #[arg(short = 't', long)]
    pub target: String,
}

#[derive(Args)]
pub struct ValidateArgs {
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    pub input: Option<std::path::PathBuf>,

    /// Input format (ac or toml). Defaults to toml.
    #[arg(short = 'f', long, default_value = "toml")]
    pub format: String,

    /// Schema file path.
    #[arg(short, long)]
    pub schema: std::path::PathBuf,
}

#[derive(Args)]
pub struct FieldsArgs {
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    pub input: Option<std::path::PathBuf>,

    /// Input format (ac or toml). Defaults to toml.
    #[arg(short = 'f', long, default_value = "toml")]
    pub format: String,
}

#[derive(Args)]
pub struct TypegenArgs {
    /// Input file path.
    #[arg(short, long)]
    pub input: std::path::PathBuf,

    /// Input format (ac or toml). Defaults to toml.
    #[arg(short = 'f', long, default_value = "toml")]
    pub format: String,

    /// Output format (json-schema). Defaults to json-schema.
    #[arg(short = 'o', long, default_value = "json-schema")]
    pub output_format: String,

    /// Print schema to stdout instead of writing files.
    #[arg(long)]
    pub stdout: bool,

    /// Output directory for generated schema files.
    #[arg(long, default_value = ".ac/generated/schemas")]
    pub output_dir: std::path::PathBuf,
}

#[derive(Args)]
pub struct TypecheckArgs {
    /// Input file path. Reads from stdin if omitted.
    #[arg(short, long)]
    pub input: Option<std::path::PathBuf>,

    /// Input format (ac or toml). Defaults to toml.
    #[arg(short = 'f', long, default_value = "toml")]
    pub format: String,
}
