use std::fs::File;
use std::io::{self,Read};

fn main() -> io::Result<()>{
    // let mut file = File::open("test.asm")?;

    // println!("File is {:?}",file);

    println!("Add meg mennyivel mentél");
    let mut input = String::new();

    
    io::stdin().read_line(&mut input)?;

    println!("You entered line{}",input);

    
    println!("A birság {}",input.trim().parse::<f32>().unwrap() / 50. * 5000.);

    Ok(())
}
