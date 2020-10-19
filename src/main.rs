//!
//! FPGRARS was made as an alternative to [RARS](https://github.com/TheThirdOne/rars), as it was
//! too slow for some applications. As such, it implements parsing and simulating RISC-V code,
//! as well as showing images on the screen and interacting with user input.
//!
//! Right now I don't aim to implement the instructions too close to what a real RISC-V processor
//! would execute. For example, there are some pseudoinstructions implemented as real instructions,
//! it's impossible to make self-modifying code and there's no difference between `jal` and `call`.
//! Even then, I think these won't make too much of a difference for most users.
//!
//! Also note that the simulator cares less about correctness than RARS, so some programs that run
//! here will fail there. One such case occurs if you read a word from an unaligned position in memory,
//! FPGRARS doesn't care, but RARS complains.
//!

mod renderer;
mod simulator;

use std::env;
use std::error::Error;
use std::thread;

fn main() -> Result<(), Box<dyn Error>> {
    let sim = simulator::Simulator::new();
    let mmio = sim.memory.mmio.clone();

    let mut args: Vec<String> = env::args().collect();
    let file = args.pop().expect("Usage: ./fpgrars [OPTIONS] riscv_file.s");

    thread::Builder::new()
        .name("FPGRARS Simulator".into())
        .spawn(move || {
            let mut sim = sim.load_from_file(file).unwrap(); // TODO: not unwrap

            for instruction in sim.code.iter() {
                println!("{:?}", instruction);
            }

            sim.run();
        })?;

    renderer::init(mmio);

    Ok(())
}
