use std::fmt;
use std::fs::File;
use std::io::{Read, Write};

use thiserror::Error;

use crate::instructions::{Binary, Prefix, Special, Unary, UserCommand};

#[derive(Clone)]
pub struct Program {
    pub(crate) code: Vec<u8>,
    pub(crate) stack_size: i32,
    pub(crate) offset: usize,
}

pub const POSTFIX_MAX: u8 = 15; // U4::MAX

#[derive(Error, Debug)]
pub enum SyntaxError {
    #[error("variable already defined: {0}")]
    RedifinedVariable(String),

    #[error("variable was not defined: {0}")]
    UndefinedVariable(String),

    #[error("cannot unnest scope without parent")]
    ConnotUnnest,

    #[error(
        "cannot {0}, postfix limit: [{1}] greater then limit [{}]",
        POSTFIX_MAX
    )]
    PostfixLimit(&'static str, u8),

    #[error("fragment in {0} cannot modify stack size")]
    FragmentCannotModifyStackSize(&'static str),

    #[error("could not parse, remainder: {0}")]
    CouldNotParseRamainder(String),
    #[error("parse error")]
    ParseError(String),
}

#[allow(dead_code)]
impl Program {
    fn write(&mut self, buffer: &[u8]) -> &mut Program {
        self.code.write_all(buffer).unwrap();
        self
    }

    pub fn from_binary(data: Vec<u8>) -> Program {
        Program {
            code: data,
            stack_size: 0,
            offset: 0,
        }
    }

    pub fn from_file(path: &str) -> std::io::Result<Program> {
        let mut stored_bin = Vec::<u8>::new();
        File::open(path)?.read_to_end(&mut stored_bin)?;
        Ok(Program {
            code: stored_bin,
            stack_size: 0,
            offset: 0,
        })
    }

    pub fn new() -> Program {
        Program {
            code: Vec::<u8>::new(),
            stack_size: 0,
            offset: 0,
        }
    }

    pub fn nop(&mut self) -> &mut Program {
        self.write(&[Prefix::POP as u8]) // POP 0
    }

    pub fn pop(&mut self, n: u8) -> Result<&mut Program, SyntaxError> {
        if n > POSTFIX_MAX {
            Err(SyntaxError::PostfixLimit("pop", n))
        } else {
            self.stack_size -= i32::from(n);
            Ok(self.write(&[Prefix::POP as u8 | n])) // POP n
        }
    }

    /* This can be used to allow fragments (i.e. in a branch arm) to modify the stack size */
    pub fn leave_on_stack(&mut self, n: i32) -> &mut Program {
        self.stack_size -= n;
        self
    }

    pub fn peek(&mut self, n: u8) -> Result<&mut Program, SyntaxError> {
        if n > POSTFIX_MAX {
            Err(SyntaxError::PostfixLimit("peek", n))
        } else {
            self.stack_size += 1;
            Ok(self.write(&[Prefix::PEEK as u8 | n])) // PEEK n
        }
    }

    pub fn swap(&mut self, n: u8) -> Result<&mut Program, SyntaxError> {
        if n > POSTFIX_MAX {
            Err(SyntaxError::PostfixLimit("swap", n))
        } else {
            Ok(self.write(&[Prefix::SWAP as u8 | n]))
        }
    }

    pub fn unary(&mut self, u: Unary) -> &mut Program {
        self.write(&[Prefix::UNARY as u8 | u as u8]) // UNARY u
    }

    pub(crate) fn binary(&mut self, u: Binary) -> &mut Program {
        self.stack_size -= 1;
        self.write(&[Prefix::BINARY as u8 | u as u8]) // BINARY u
    }

    pub fn special(&mut self, u: Special) -> &mut Program {
        self.stack_size += match u {
            Special::DUMP => 0,
            Special::TWOBYTE => unimplemented!(),
        };
        self.write(&[Prefix::SPECIAL as u8 | u as u8]) // SPECIAL u
    }

    pub fn user(&mut self, u: UserCommand) -> &mut Program {
        self.stack_size += match u {
            UserCommand::GET_LENGTH => 1,
            UserCommand::GET_PRECISE_TIME => 1,
            UserCommand::GET_WALL_TIME => 1,
            UserCommand::BLIT => 0,
            UserCommand::SET_PIXEL => -1,
            UserCommand::RANDOM_INT => 0,
            UserCommand::GET_PIXEL => 0,
        };
        self.write(&[Prefix::USER as u8 | u as u8]) // SPECIAL u
    }

    fn skip<F>(&mut self, prefix: Prefix, mut builder: F) -> Result<&mut Program, SyntaxError>
    where
        F: FnMut(&mut Program) -> Result<(), SyntaxError>,
    {
        let mut fragment = Program {
            code: Vec::<u8>::new(),
            stack_size: 0,
            offset: self.current_pc() + 3, // before fragment would be inst+2bytes address
        };
        builder(&mut fragment)?;
        if fragment.stack_size != 0 {
            return Err(SyntaxError::FragmentCannotModifyStackSize("branch"));
        }

        // [JS/JNS, addr, addr, ...fragment], so we add 3 on top of fragment size to get end addr
        let end_address = self.current_pc() + 3 + fragment.code.len();
        // Always write three-byte jumps for now
        self.write(&[
            prefix as u8,
            (end_address & 0xFF) as u8,
            ((end_address >> 8) & 0xFF) as u8,
        ]);
        self.write(&fragment.code);
        Ok(self)
    }

    pub fn if_zero<F>(&mut self, builder: F) -> Result<&mut Program, SyntaxError>
    where
        F: FnMut(&mut Program) -> Result<(), SyntaxError>,
    {
        self.skip(Prefix::JNZ, builder)
    }

    pub fn if_not_zero<F>(&mut self, builder: F) -> Result<&mut Program, SyntaxError>
    where
        F: FnMut(&mut Program) -> Result<(), SyntaxError>,
    {
        self.skip(Prefix::JZ, builder)
    }

    pub fn repeat_forever<F>(&mut self, mut builder: F) -> Result<&mut Program, SyntaxError>
    where
        F: FnMut(&mut Program) -> Result<(), SyntaxError>,
    {
        let mut fragment = Program {
            code: Vec::<u8>::new(),
            stack_size: 0,
            offset: self.current_pc(),
        };
        builder(&mut fragment)?;
        if fragment.stack_size != 0 {
            return Err(SyntaxError::FragmentCannotModifyStackSize("forever loop"));
        }

        let start = self.current_pc();
        self.write(&fragment.code);
        self.write(&[
            Prefix::JMP as u8,
            (start & 0xFF) as u8,
            ((start >> 8) & 0xFF) as u8,
        ]);
        Ok(self)
    }

    fn current_pc(&self) -> usize {
        self.offset + self.code.len()
    }

    pub fn repeat<F>(&mut self, mut builder: F) -> Result<&mut Program, SyntaxError>
    where
        F: FnMut(&mut Program) -> Result<(), SyntaxError>,
    {
        let mut fragment = Program {
            code: Vec::<u8>::new(),
            stack_size: 0,
            offset: self.current_pc() + 3, // before fragment would be inst+2bytes address
        };
        builder(&mut fragment)?;
        if fragment.stack_size != 0 {
            return Err(SyntaxError::FragmentCannotModifyStackSize("for loop"));
        }

        let start = self.current_pc();
        // [JMP,addr,addr][...loop body...][DEC][JMP,addr,addr]
        let end = start + 3 + fragment.code.len() + 1 + 3;
        self.write(&[
            Prefix::JZ as u8,
            (end & 0xFF) as u8,
            ((end >> 8) & 0xFF) as u8,
        ]);

        self.write(&fragment.code);
        self.write(&[Prefix::UNARY as u8 | Unary::DEC as u8]);
        self.write(&[
            Prefix::JMP as u8,
            (start & 0xFF) as u8,
            ((start >> 8) & 0xFF) as u8,
        ]);
        Ok(self)
    }

    pub fn repeat_times<F>(&mut self, times: u32, builder: F) -> Result<&mut Program, SyntaxError>
    where
        F: FnMut(&mut Program) -> Result<(), SyntaxError>,
    {
        self.push(times);
        self.repeat(builder)?;
        self.pop(1)
    }

    pub fn inc(&mut self) -> &mut Program {
        self.unary(Unary::INC)
    }

    pub fn dec(&mut self) -> &mut Program {
        self.unary(Unary::DEC)
    }

    pub fn not(&mut self) -> &mut Program {
        self.unary(Unary::NOT)
    }

    pub fn neg(&mut self) -> &mut Program {
        self.unary(Unary::NEG)
    }

    pub fn add(&mut self) -> &mut Program {
        self.binary(Binary::ADD)
    }

    pub fn and(&mut self) -> &mut Program {
        self.binary(Binary::AND)
    }

    pub fn div(&mut self) -> &mut Program {
        self.binary(Binary::DIV)
    }

    pub fn gt(&mut self) -> &mut Program {
        self.binary(Binary::GT)
    }

    pub fn gte(&mut self) -> &mut Program {
        self.binary(Binary::GTE)
    }

    pub fn lt(&mut self) -> &mut Program {
        self.binary(Binary::LT)
    }

    pub fn lte(&mut self) -> &mut Program {
        self.binary(Binary::LTE)
    }

    pub fn r#mod(&mut self) -> &mut Program {
        self.binary(Binary::MOD)
    }

    pub fn mul(&mut self) -> &mut Program {
        self.binary(Binary::MUL)
    }

    pub fn or(&mut self) -> &mut Program {
        self.binary(Binary::OR)
    }

    pub fn sub(&mut self) -> &mut Program {
        self.binary(Binary::SUB)
    }

    pub fn xor(&mut self) -> &mut Program {
        self.binary(Binary::XOR)
    }

    pub fn dump(&mut self) -> &mut Program {
        self.special(Special::DUMP)
    }

    pub fn dup(&mut self) -> Result<&mut Program, SyntaxError> {
        self.peek(0)
    }

    pub fn set_pixel(&mut self) -> &mut Program {
        self.user(UserCommand::SET_PIXEL)
    }

    pub fn blit(&mut self) -> &mut Program {
        self.user(UserCommand::BLIT)
    }

    pub fn get_length(&mut self) -> &mut Program {
        self.user(UserCommand::GET_LENGTH)
    }

    pub fn get_precise_time(&mut self) -> &mut Program {
        self.user(UserCommand::GET_PRECISE_TIME)
    }

    pub fn get_wall_time(&mut self) -> &mut Program {
        self.user(UserCommand::GET_WALL_TIME)
    }

    pub fn push(&mut self, b: u32) -> &mut Program {
        self.stack_size += 1;
        match b {
            0 => self.code.write(&[Prefix::PUSHB as u8]).unwrap(),
            _ if b <= 0xFF => self
                .code
                .write(&[Prefix::PUSHB as u8 | 0x01, b as u8])
                .unwrap(),
            _ => self
                .code
                .write(&[
                    Prefix::PUSHI as u8 | 0x01,
                    (b & 0xFF) as u8,
                    ((b >> 8) & 0xFF) as u8,
                    ((b >> 16) & 0xFF) as u8,
                    ((b >> 24) & 0xFF) as u8,
                ])
                .unwrap(),
        };
        self
    }

    pub fn code(&self) -> &Vec<u8> {
        &self.code
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pc = 0;
        while pc < self.code.len() {
            let ins = Prefix::from(self.code[pc]);
            if let Some(i) = ins {
                let postfix = self.code[pc] & 0x0F;
                write!(f, "{:04}.\t{:02x}\t{}", pc, self.code[pc], i)?;
                match i {
                    Prefix::PUSHI => {
                        let end = (postfix as usize) * 4 + pc + 1;
                        if end > self.code.len() {
                            write!(f, "\t(invalid, overruns code; size={})", (postfix as usize))?;
                            return Ok(());
                        } else {
                            write!(
                                f,
                                "\t{:02x?}",
                                &self.code[(pc + 1)..(pc + 1 + (postfix as usize) * 4)]
                            )?;
                            pc += (postfix as usize) * 4;
                        }
                    }
                    Prefix::PUSHB => {
                        if postfix == 0 {
                            write!(f, "\t0")?;
                        } else {
                            let end = (postfix as usize) + pc + 1;
                            if end > self.code.len() {
                                write!(
                                    f,
                                    "\t(invalid, overruns code; size={})",
                                    (postfix as usize)
                                )?;
                                return Ok(());
                            } else {
                                write!(
                                    f,
                                    "\t{:02x?}",
                                    &self.code[(pc + 1)..(pc + 1 + (postfix as usize))]
                                )?;
                                pc += postfix as usize;
                            }
                        }
                    }
                    Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
                        if self.code.len() < (pc + 1) {
                            write!(f, "\t(invalid, overruns code)")?;
                            return Ok(());
                        }
                        let target =
                            u32::from(self.code[pc + 1]) | u32::from(self.code[pc + 2]) << 8;
                        write!(f, "\tto {}", target)?;
                        pc += 2
                    }
                    Prefix::BINARY => {
                        if let Some(op) = Binary::from(postfix) {
                            write!(f, "\t{}", op)?;
                        } else {
                            write!(f, "\tunknown {}", postfix)?;
                        }
                    }
                    Prefix::UNARY => {
                        if let Some(op) = Unary::from(postfix) {
                            write!(f, "\t{}", op)?;
                        } else {
                            write!(f, "\tunknown {}", postfix)?;
                        }
                    }
                    Prefix::USER => {
                        let name = match postfix {
                            0 => "get_length",
                            1 => "get_wall_time",
                            2 => "get_precise_time",
                            3 => "set_pixel",
                            4 => "blit",
                            5 => "random_int",
                            6 => "get_pixel",
                            _ => "(unknown user function)",
                        };
                        write!(f, "\t{}", name)?;
                    }
                    Prefix::SPECIAL => {
                        let name = match postfix {
                            12 => "swap",
                            13 => "dump",
                            14 => "yield",
                            15 => "two-byte instruction",
                            _ => "(unknown special function)",
                        };
                        write!(f, "\t{}", name)?;
                    }
                    _ => {
                        write!(f, "\t{}", postfix)?;
                    }
                }
                writeln!(f)?;
            } else {
                writeln!(f, "{:04}.\t{:02x}\tUnknown instruction", pc, self.code[pc])?;
                break;
            }

            pc += 1;
        }
        Ok(())
    }
}
