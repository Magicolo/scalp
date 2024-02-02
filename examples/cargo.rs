use scalp::*;

pub struct Cargo {}

fn main() -> Result<(), Error> {
    let parser = Builder::new()
        .name("git")
        .version(env!("CARGO_PKG_VERSION"))
        .map(|()| Cargo {})
        .build()?;
    let arguments = ["--help"];
    let environment = [("", "")];
    Ok(())
}
