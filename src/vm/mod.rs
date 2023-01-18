pub mod errors;
pub(crate) mod strip;

use super::instructions::{Binary, Prefix, Special, Unary, UserCommand};
use crate::program::Program;
use derivative::Derivative;
use errors::VMError;
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rgb::RGB8;
use std::time::{SystemTime, UNIX_EPOCH};
use strip::DummyLedStrip;

#[derive(Derivative)]
#[derivative(Default)]
pub struct VMStateConfig {
    pub global_instruction_limit: Option<usize>,
    pub local_instruction_limit: Option<usize>,
    #[derivative(Default(value = "Box::new(ChaCha8Rng::seed_from_u64(0))"))]
    pub rng: Box<dyn RngCore>,
}

pub struct VMState {
    vm: VM,
    program: Program,
    pc: usize,
    stack: Vec<u32>,
    start_time: SystemTime,
    instruction_count: usize,
    config: VMStateConfig,
}

pub struct VM {
    strip: DummyLedStrip,
    pub config: VMConfig,
}

#[derive(Default)]
pub struct VMConfig {
    pub trace: bool,
    pub deterministic: bool,
}

pub enum Outcome {
    Ended,
    Error(VMError),
    BLIT(Box<dyn Iterator<Item = RGB8> + Send>),
}

impl VMState {
    fn new(vm: VM, program: Program, config: VMStateConfig) -> VMState {
        let start_time = if vm.config.deterministic {
            SystemTime::UNIX_EPOCH
        } else {
            SystemTime::now()
        };
        VMState {
            vm,
            program,
            pc: 0,
            stack: vec![],
            start_time,
            config,
            instruction_count: 0,
        }
    }
    pub fn pc(&self) -> usize {
        self.pc
    }

    fn pushi(&mut self, postfix: u8) {
        for _ in 0..postfix {
            let value = u32::from(self.program.code[self.pc + 1])
                | u32::from(self.program.code[self.pc + 2]) << 8
                | u32::from(self.program.code[self.pc + 3]) << 16
                | u32::from(self.program.code[self.pc + 4]) << 24;
            self.stack.push(value);

            if self.vm.config.trace {
                print!("\tv={}", value);
            }
            self.pc += 4;
        }
    }

    fn pushb(&mut self, postfix: u8) {
        if postfix == 0 {
            self.stack.push(0);
        } else {
            for _ in 0..postfix {
                self.pc += 1;
                if self.vm.config.trace {
                    print!("\tv={}", self.program.code[self.pc]);
                }
                self.stack.push(u32::from(self.program.code[self.pc]));
            }
        }
    }

    fn user(&mut self, postfix: u8) -> Option<Outcome> {
        let user = UserCommand::from(postfix);

        match user {
            None => Some(Outcome::Error(VMError::UnknownInstruction(postfix))),
            Some(UserCommand::GET_LENGTH) => {
                self.stack.push(self.vm.strip.length() as u32);
                None
            }
            Some(UserCommand::GET_WALL_TIME) => {
                if self.vm.config.deterministic {
                    self.stack.push((self.instruction_count / 10) as u32);
                } else {
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    self.stack.push((time & u32::MAX as u64) as u32); // Wrap around when we exceed u32::MAX
                }
                None
            }
            Some(UserCommand::GET_PRECISE_TIME) => {
                if self.vm.config.deterministic {
                    self.stack.push(self.instruction_count as u32);
                } else {
                    let time = SystemTime::now()
                        .duration_since(self.start_time)
                        .unwrap()
                        .as_millis();
                    self.stack.push((time & u32::MAX as u128) as u32); // Wrap around when we exceed u32::MAX
                }
                None
            }
            Some(UserCommand::SET_PIXEL) => {
                if let (Some(v), Some(idx)) = (self.stack.pop(), self.stack.last()) {
                    let [r, g, b, _] = v.to_le_bytes();
                    let color = RGB8::new(r, g, b);

                    if self.vm.config.trace {
                        print!("\tset_pixel {} idx={} color={:?}", v, idx, color);
                    }

                    if *idx >= self.vm.strip.length() {
                        return Some(Outcome::Error(VMError::RuntimeError(format!(
                            "index {} exceeds strip length {}",
                            *idx,
                            self.vm.strip.length()
                        ))));
                    }

                    self.vm.strip.set_pixel(*idx, color);
                    None
                } else {
                    Some(Outcome::Error(VMError::StackUnderflow))
                }
            }
            Some(UserCommand::BLIT) => {
                if self.vm.config.trace {
                    print!("\tblit");
                }
                self.vm.strip.blit();
                self.pc += 1;
                Some(Outcome::BLIT(self.vm.strip.export()))
            }
            Some(UserCommand::RANDOM_INT) => {
                if let Some(v) = self.stack.pop() {
                    self.stack.push(self.config.rng.gen_range(0..v));
                    None
                } else {
                    Some(Outcome::Error(VMError::StackUnderflow))
                }
            }
            Some(UserCommand::GET_PIXEL) => {
                if let Some(v) = self.stack.pop() {
                    let color = self.vm.strip.get_pixel(v);
                    let color_value = u32::from_le_bytes([color.r, color.g, color.b, 0]);
                    self.stack.push(color_value);
                    None
                } else {
                    Some(Outcome::Error(VMError::StackUnderflow))
                }
            }
        }
    }

    fn special(&mut self, postfix: u8) -> Option<Outcome> {
        let special = Special::from(postfix);

        match special {
            None => Some(Outcome::Error(VMError::UnknownInstruction(postfix))),
            Some(Special::DUMP) => {
                // DUMP
                println!("DUMP: {:?}", self.stack);
                None
            }
            Some(Special::TWOBYTE) => {
                Some(Outcome::Error(VMError::UnimplementedInstruction(postfix)))
            }
        }
    }

    pub fn run(&mut self) -> Outcome {
        let mut local_instruction_count = 0;
        while self.pc < self.program.code.len() {
            // Enforce global instruction count limit
            if let Some(limit) = self.config.global_instruction_limit {
                if self.instruction_count >= limit {
                    return Outcome::Error(VMError::GlobalInstructionLimitReached(
                        self.instruction_count,
                        limit,
                    ));
                }
            }

            // Enforce local instruction count limit
            if let Some(limit) = self.config.local_instruction_limit {
                if local_instruction_count >= limit {
                    return Outcome::Error(VMError::LocalInstructionLimitReached(
                        local_instruction_count,
                        limit,
                    ));
                }
            }

            let ins = Prefix::from(self.program.code[self.pc]);
            if let Some(i) = ins {
                self.instruction_count += 1;
                local_instruction_count += 1;
                let postfix = self.program.code[self.pc] & 0x0F;

                if self.vm.config.trace {
                    print!("{:04}.\t{:02x}\t{}", self.pc, self.program.code[self.pc], i);
                }

                match i {
                    Prefix::PUSHI => {
                        self.pushi(postfix);
                    }
                    Prefix::PUSHB => {
                        self.pushb(postfix);
                    }
                    Prefix::POP => {
                        if postfix as usize > self.stack.len() {
                            return Outcome::Error(VMError::StackUnderflow);
                        }

                        for _ in 0..postfix {
                            let _ = self.stack.pop();
                        }
                    }
                    Prefix::PEEK => {
                        if postfix as usize >= self.stack.len() {
                            return Outcome::Error(VMError::StackUnderflow);
                        }
                        let val = self.stack[self.stack.len() - (postfix as usize) - 1];
                        if self.vm.config.trace {
                            print!("\tindex={} v={}", postfix, val);
                        }
                        self.stack.push(val);
                    }
                    Prefix::SWAP => {
                        if postfix as usize >= self.stack.len() {
                            return Outcome::Error(VMError::StackUnderflow);
                        }
                        let last_i = self.stack.len() - 1;
                        let target_i = last_i - (postfix as usize);
                        self.stack.swap(target_i, last_i);
                    }
                    Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
                        let target = (u32::from(self.program.code[self.pc + 1])
                            | (u32::from(self.program.code[self.pc + 2]) << 8))
                            as usize;

                        self.pc = match i {
                            Prefix::JMP => target,
                            Prefix::JZ => {
                                if let Some(head) = self.stack.last() {
                                    if *head == 0 {
                                        target
                                    } else {
                                        self.pc + 3
                                    }
                                } else {
                                    return Outcome::Error(VMError::StackUnderflow);
                                }
                            }
                            Prefix::JNZ => {
                                if let Some(head) = self.stack.last() {
                                    if *head != 0 {
                                        target
                                    } else {
                                        self.pc + 3
                                    }
                                } else {
                                    return Outcome::Error(VMError::StackUnderflow);
                                }
                            }
                            _ => unreachable!(),
                        };

                        if self.vm.config.trace {
                            println!();
                        }
                        continue;
                    }
                    Prefix::BINARY => {
                        if let Some(op) = Binary::from(postfix) {
                            if let (Some(rhs), Some(lhs)) = (self.stack.pop(), self.stack.pop()) {
                                match op.apply(lhs, rhs) {
                                    Ok(v) => self.stack.push(v),
                                    Err(e) => return Outcome::Error(e),
                                }
                            } else {
                                return Outcome::Error(VMError::StackUnderflow);
                            }
                        } else {
                            return Outcome::Error(VMError::UnknownInstruction(postfix));
                        }
                    }
                    Prefix::UNARY => {
                        if let Some(op) = Unary::from(postfix) {
                            if let Some(lhs) = self.stack.pop() {
                                self.stack.push(op.apply(lhs))
                            } else {
                                return Outcome::Error(VMError::StackUnderflow);
                            }
                        } else {
                            return Outcome::Error(VMError::UnknownInstruction(postfix));
                        }
                    }
                    Prefix::USER => {
                        if let Some(outcome) = self.user(postfix) {
                            return outcome;
                        }
                    }
                    Prefix::SPECIAL => {
                        if let Some(outcome) = self.special(postfix) {
                            return outcome;
                        }
                    }
                }
            } else {
                return Outcome::Error(VMError::UnknownInstruction(self.program.code[self.pc]));
            }

            if self.vm.config.trace {
                println!("\tstack: {:?}", self.stack);
            }
            self.pc += 1;
        }

        if self.vm.config.trace {
            println!("Ended; {} instructions executed", self.instruction_count);
        }

        Outcome::Ended
    }

    pub fn stop(self) -> (VM, VMStateConfig, Program) {
        (self.vm, self.config, self.program)
    }
}

impl VM {
    pub fn new(length: usize, config: VMConfig) -> VM {
        VM {
            strip: DummyLedStrip::new(length),
            config,
        }
    }

    pub fn set_stip_length(&mut self, length: usize) {
        self.strip.set_length(length)
    }

    pub fn start(self, program: Program, config: VMStateConfig) -> VMState {
        if self.config.trace {
            println!("prog hex dump: {:X?}", program.code)
        }
        VMState::new(self, program, config)
    }
}

impl Iterator for VMState {
    type Item = Result<Box<dyn Iterator<Item = RGB8> + Send>, VMError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.run() {
            Outcome::Ended => None,
            Outcome::BLIT(iter) => Some(Ok(iter)),
            Outcome::Error(e) => Some(Err(e)),
        }
    }
}
