use potty::{Pot};
use std::str;
use std::fs::File;
use std::io::{Cursor};
use std::io::{BufReader, Result};

fn main() -> Result<()> {
    let file = File::open("example.po")?;
    let mut reader = BufReader::new(file);
    let pot = Pot::read(&mut reader);
    let mut w = Cursor::new(Vec::new());
    pot.write(&mut w)?;
    println!("{}", str::from_utf8(w.get_ref()).unwrap());
    Ok(())
}