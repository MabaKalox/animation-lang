use thiserror::Error;

#[derive(Error, Debug)]
pub enum VMError {
    #[error("global Instruction limit reached: cur[{0}] > max[{1}]")]
    GlobalInstructionLimitReached(usize, usize),

    #[error("local Instruction limit reached: cur[{0}] > max[{1}]")]
    LocalInstructionLimitReached(usize, usize),

    #[error("unknow instruction")]
    UnknownInstruction,

    #[error("stack under flow")]
    StackUnderflow,

    #[error("run time error: {0}")]
    RuntimeError(String),
}
