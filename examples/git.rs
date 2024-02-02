use scalp::*;

pub struct Git {}

fn main() -> Result<(), Error> {
    let parser = Builder::new()
        .name("git")
        .version(env!("CARGO_PKG_VERSION"))
        .map(|()| Git {})
        .build()?;
    let arguments = ["--help"];
    let environment = [("", "")];
    Ok(())
}
