mod cli;
mod decoder;
mod validator;

fn main() -> anyhow::Result<()> {
    cli::main_inner()
}
