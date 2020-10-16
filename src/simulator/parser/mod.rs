//!
//! Parses RISC-V code into code and data parts, so it can be used in the simulator module.
//! We use a lot of mnemonics here, I'll try to link to a cheatsheet here later.
//!

use radix_trie::Trie;

mod register_names;
use register_names::{self as reg_names, RegMap};

mod combinators;
use combinators::*;

mod preprocessor;
pub use preprocessor::*;

mod util;
pub use util::*;

/// Giant enum that represents a single RISC-V instruction and its arguments
#[allow(dead_code)] // please, cargo, no more warnings
#[derive(Debug)]
pub enum Instruction {
    // Type R
    /// rd, rs1, rs2
    Add(u8, u8, u8),
    Sub(u8, u8, u8),
    Sll(u8, u8, u8),
    Slt(u8, u8, u8),
    Sltu(u8, u8, u8),
    Xor(u8, u8, u8),
    Srl(u8, u8, u8),
    Sra(u8, u8, u8),
    Or(u8, u8, u8),
    And(u8, u8, u8),
    Mul(u8, u8, u8), // TODO: mulh, mulhsu, mulhu
    Div(u8, u8, u8),
    Divu(u8, u8, u8),
    Rem(u8, u8, u8),
    Remu(u8, u8, u8),

    // Type I
    Ecall,
    /// rd, imm, rs1
    Lb(u8, i32, u8),
    Lh(u8, i32, u8),
    Lw(u8, i32, u8),
    Lbu(u8, i32, u8),
    Lhu(u8, i32, u8),
    Addi(u8, u8, i32),
    /// rd, rs1, imm
    Slti(u8, u8, i32),
    Sltiu(u8, u8, u32),
    Slli(u8, u8, i32),
    Srli(u8, u8, i32),
    Srai(u8, u8, i32),
    Ori(u8, u8, u32),
    Andi(u8, u8, u32),
    Xori(u8, u8, u32),

    // Type S
    /// rs2, imm, rs1
    Sb(u8, i32, u8),
    Sh(u8, i32, u8),
    Sw(u8, i32, u8),

    // Type SB + jumps
    /// rs1, rs2, label
    Beq(u8, u8, usize),
    Bne(u8, u8, usize),
    Blt(u8, u8, usize),
    Bge(u8, u8, usize),
    Bltu(u8, u8, usize),
    Bgeu(u8, u8, usize),
    /// rd, rs1, imm
    Jalr(u8, u8, i32),
    /// rd, label
    Jal(u8, usize),

    // Some pseudoinstructions
    /// rd, imm
    Li(u8, i32),
    /// rd, rs1
    Mv(u8, u8),
    /// rd, label
    La(u8, usize),

    Ret,
}

/// Also giant enum that represents a single RISC-V instruction, but we save
/// labels as strings because it might not have parsed it yet (for example,
/// consider a jump instruction that jumps to a label in the next line).
///
/// We process the labels stored after the entire file has been parsed.
enum PreLabelInstruction {
    Beq(u8, u8, String),
    Bne(u8, u8, String),
    Blt(u8, u8, String),
    Bge(u8, u8, String),
    Bltu(u8, u8, String),
    Bgeu(u8, u8, String),
    Jal(u8, String),
    La(u8, String),
    Other(Instruction),
}

impl From<Instruction> for PreLabelInstruction {
    fn from(i: Instruction) -> PreLabelInstruction {
        PreLabelInstruction::Other(i)
    }
}

/// Represents a successful parser result. This is the same format the simulator
/// will use to execute the instructions
pub struct Parsed {
    pub code: Vec<Instruction>,
    pub data: Vec<u8>,
}

pub type ParseResult = Result<Parsed, Error>;

/// The "current" parser directive
enum Directive {
    Text,
    Data,
}

pub trait RISCVParser {
    /// Parses an iterator of preprocessed lines and returns the instructions and
    /// the data it parsed. Remember to preprocess the iterator before calling this,
    /// as `parse_riscv` does not understand macros and includes.
    /// ```
    /// parse::file_lines("riscv.s".to_owned())?
    ///     .parse_includes()
    ///     .parse_macros()
    ///     .parse_riscv(DATA_SIZE)?;
    /// ```
    ///
    /// The `data_segment_size` parameter is the final size of the data segment, in bytes.
    fn parse_riscv(self, data_segment_size: usize) -> ParseResult;
}

type FullRegMap = (RegMap, RegMap, RegMap);

impl<I: Iterator<Item = String>> RISCVParser for I {
    fn parse_riscv(self, data_segment_size: usize) -> ParseResult {
        use combinators::*;

        let regmaps = (reg_names::regs(), reg_names::floats(), reg_names::status());
        let mut labels = Trie::<String, usize>::new();

        let mut directive = Directive::Text;
        let mut code = Vec::new();
        let mut data = Vec::with_capacity(data_segment_size);

        for line in self {
            // TODO: extract this into a function
            let line = match parse_label(&line) {
                Ok((rest, label)) => {
                    let label_pos = match directive {
                        Directive::Text => code.len() * 4,
                        Directive::Data => data.len(),
                    };
                    labels.insert(label.to_owned(), label_pos);
                    rest
                }
                Err(_) => &line,
            };

            if line.is_empty() {
                continue;
            }

            // Identify directives
            // This accepts stuff like ".textSOMETHING" or ".database", but RARS accepts it too
            // Gotta be consistent! ¯\_(ツ)_/¯
            if line.starts_with(".data") {
                directive = Directive::Data;
                continue;
            } else if line.starts_with(".text") {
                directive = Directive::Text;
                continue;
            }

            match directive {
                Directive::Text => code.push(parse_text(line, &regmaps)?),
                Directive::Data => unimplemented!("No .data implementation yet"),
            }

            println!("> {}", line);
        }

        let code: Result<Vec<Instruction>, Error> = code
            .into_iter()
            .map(|i| unlabel_instruction(i, &labels))
            .collect();
        let mut code = code?;

        code.push(Instruction::Jal(0, code.len() * 4));

        // If the program ever drops off bottom, we make an "exit" ecall and terminate execution
        code.extend(vec![
            Instruction::Li(17, 10), // li a7 10
            Instruction::Ecall,
        ]);

        data.resize(data_segment_size, 0);
        Ok(Parsed { code, data })
    }
}

fn parse_text(s: &str, regmaps: &FullRegMap) -> Result<PreLabelInstruction, Error> {
    let (regs, floats, status) = regmaps;
    use Instruction::*;
    use PreLabelInstruction as pre;

    macro_rules! type_r {
        ($inst:expr) => {
            args_type_r(s, &regs).map(|(rd, rs1, rs2)| $inst(rd, rs1, rs2).into())?
        };
    }

    macro_rules! type_sb {
        ($inst:expr) => {
            args_type_sb(s, &regs).map(|(rs1, rs2, label)| $inst(rs1, rs2, label))?
        };
    }

    // Reverses the order of rs1 and rs2 to convert, for example,
    // `ble t0 t1 label` into `bge t1 t0 label`
    macro_rules! type_sb_reversed {
        ($inst:expr) => {
            args_type_sb(s, &regs).map(|(rs1, rs2, label)| $inst(rs2, rs1, label))?
        };
    }

    let (s, instruction) = one_arg(s)?;

    let parsed = match instruction.to_lowercase().as_str() {
        "add" => type_r!(Add),
        "sub" => type_r!(Sub),
        "sll" => type_r!(Sll),
        "slt" => type_r!(Slt),
        "sltu" => type_r!(Sltu),
        "xor" => type_r!(Xor),
        "srl" => type_r!(Srl),
        "sra" => type_r!(Sra),
        "or" => type_r!(Or),
        "and" => type_r!(And),

        "beq" => type_sb!(pre::Beq),
        "bne" => type_sb!(pre::Bne),
        "blt" => type_sb!(pre::Blt),
        "bge" => type_sb!(pre::Bge),
        "bltu" => type_sb!(pre::Bltu),
        "bgeu" => type_sb!(pre::Bgeu),
        "bgt" => type_sb_reversed!(pre::Blt),
        "ble" => type_sb_reversed!(pre::Bge),
        "bgtu" => type_sb_reversed!(pre::Bltu),
        "bleu" => type_sb_reversed!(pre::Bgeu),

        "jal" => args_jal(s, &regs).map(|(rd, label)| pre::Jal(rd, label))?,
        "j" => one_arg(s).map(|(_i, label)| pre::Jal(0, label.to_owned()))?,

        "ecall" => Ecall.into(),

        idk => unimplemented!("Instruction <{}> hasn't been implemented", idk),
    };

    Ok(parsed)
}

/// Transforms a PreLabelInstruction into a normal Instruction by "commiting" the labels
/// into positions in the code. For example, Jal(0, "Label") maps to Jal(0, labels_trie.get("Label"))
fn unlabel_instruction(
    instruction: PreLabelInstruction,
    labels: &Trie<String, usize>,
) -> Result<Instruction, Error> {
    use Instruction::*;
    use PreLabelInstruction as p;

    // TODO: refactor this, maybe
    match instruction {
        p::Jal(rd, label) => labels
            .get(&label)
            .map(|&pos| Jal(rd, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Beq(rs1, rs2, label) => labels
            .get(&label)
            .map(|&pos| Beq(rs1, rs2, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Bne(rs1, rs2, label) => labels
            .get(&label)
            .map(|&pos| Bne(rs1, rs2, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Bge(rs1, rs2, label) => labels
            .get(&label)
            .map(|&pos| Bge(rs1, rs2, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Blt(rs1, rs2, label) => labels
            .get(&label)
            .map(|&pos| Blt(rs1, rs2, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Bltu(rs1, rs2, label) => labels
            .get(&label)
            .map(|&pos| Bltu(rs1, rs2, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Bgeu(rs1, rs2, label) => labels
            .get(&label)
            .map(|&pos| Bgeu(rs1, rs2, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::La(rd, label) => labels
            .get(&label)
            .map(|&pos| La(rd, pos))
            .ok_or(Error::LabelNotFound(label)),
        p::Other(instruction) => Ok(instruction),
    }
}
